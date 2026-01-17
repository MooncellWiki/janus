use anyhow::{Context, Result};
use chrono::Utc;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Aliyun OpenAPI V3 signature generator (ACS3-HMAC-SHA256)
///
/// Docs: https://help.aliyun.com/zh/sdk/product-overview/v3-request-structure-and-signature
pub struct AliyunSigner {
    access_key_id: String,
    access_key_secret: String,
}

pub struct AliyunSignInput<'a> {
    pub method: &'a str,
    pub host: &'a str,
    pub canonical_uri: &'a str,
    pub action: &'a str,
    pub version: &'a str,
    pub query_params: BTreeMap<String, String>,
    pub body: &'a [u8],
    pub content_type: Option<&'a str>,
    /// Any extra request headers. If the name is `x-acs-*`, `host`, or `content-type`, it will be included in the signature.
    pub extra_headers: BTreeMap<String, String>,
}

pub struct AliyunSignedRequest {
    /// RFC3986-encoded canonical query string.
    pub query_string: String,
    pub headers: reqwest::header::HeaderMap,
}

impl AliyunSigner {
    pub fn new(access_key_id: String, access_key_secret: String) -> Self {
        Self {
            access_key_id,
            access_key_secret,
        }
    }

    /// Generate a random nonce (hex string) for request.
    fn generate_nonce() -> String {
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex_encode_lower(&bytes)
    }

    /// Get current timestamp in ISO 8601 format (UTC)
    fn get_timestamp() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    fn build_canonical_query_string(params: &BTreeMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    percent_encode(k.as_bytes(), NON_ALPHANUMERIC),
                    percent_encode(v.as_bytes(), NON_ALPHANUMERIC)
                )
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Canonicalize a request path (CanonicalURI).
    ///
    /// For RPC-style APIs this is typically just `/`.
    fn canonicalize_uri(path: &str) -> String {
        if path.is_empty() {
            return "/".to_string();
        }
        if path == "/" {
            return "/".to_string();
        }

        let has_trailing_slash = path.ends_with('/');
        let trimmed = path.trim_matches('/');
        let mut out = String::from("/");
        if !trimmed.is_empty() {
            out.push_str(
                &trimmed
                    .split('/')
                    .map(|segment| percent_encode(segment.as_bytes(), NON_ALPHANUMERIC).to_string())
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
        if has_trailing_slash {
            out.push('/');
        }
        out
    }

    pub fn sign_request(&self, input: AliyunSignInput<'_>) -> Result<AliyunSignedRequest> {
        let host = input.host.trim();
        let action = input.action.trim();
        let version = input.version.trim();

        let x_acs_date = Self::get_timestamp();
        let x_acs_signature_nonce = Self::generate_nonce();
        let x_acs_content_sha256 = sha256_hex(input.body);

        // Canonical query
        let canonical_query = Self::build_canonical_query_string(&input.query_params);
        let canonical_uri = Self::canonicalize_uri(input.canonical_uri);

        // Build headers participating in signing.
        // Must include: host + all x-acs-* headers (except Authorization). content-type is included if present.
        let mut signing_headers: BTreeMap<String, String> = BTreeMap::new();

        for (k, v) in input.extra_headers {
            let key = k.trim().to_ascii_lowercase();
            if key == "host" || key == "content-type" || key.starts_with("x-acs-") {
                signing_headers.insert(key, v.trim().to_string());
            }
        }

        signing_headers.insert("host".to_string(), host.to_string());
        signing_headers.insert("x-acs-action".to_string(), action.to_string());
        signing_headers.insert("x-acs-version".to_string(), version.to_string());
        signing_headers.insert("x-acs-date".to_string(), x_acs_date.clone());
        signing_headers.insert(
            "x-acs-signature-nonce".to_string(),
            x_acs_signature_nonce.clone(),
        );
        signing_headers.insert(
            "x-acs-content-sha256".to_string(),
            x_acs_content_sha256.clone(),
        );

        if let Some(ct) = input.content_type {
            signing_headers.insert("content-type".to_string(), ct.trim().to_string());
        }

        let canonical_headers = signing_headers
            .iter()
            .map(|(k, v)| format!("{}:{}\n", k, v.trim()))
            .collect::<String>();
        let signed_headers = signing_headers
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(";");

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            input.method.to_uppercase(),
            canonical_uri,
            canonical_query,
            canonical_headers,
            signed_headers,
            x_acs_content_sha256
        );

        let hashed_canonical_request = sha256_hex(canonical_request.as_bytes());
        let string_to_sign = format!("ACS3-HMAC-SHA256\n{}", hashed_canonical_request);
        let signature = hmac_sha256_hex(&self.access_key_secret, &string_to_sign);

        let authorization = format!(
            "ACS3-HMAC-SHA256 Credential={},SignedHeaders={},Signature={}",
            self.access_key_id, signed_headers, signature
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::HOST,
            host.parse().context("invalid host header value")?,
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-acs-action"),
            action
                .parse()
                .context("invalid x-acs-action header value")?,
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-acs-version"),
            version
                .parse()
                .context("invalid x-acs-version header value")?,
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-acs-date"),
            x_acs_date
                .parse()
                .context("invalid x-acs-date header value")?,
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-acs-signature-nonce"),
            x_acs_signature_nonce
                .parse()
                .context("invalid x-acs-signature-nonce header value")?,
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-acs-content-sha256"),
            x_acs_content_sha256
                .parse()
                .context("invalid x-acs-content-sha256 header value")?,
        );
        if let Some(ct) = input.content_type {
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                ct.parse().context("invalid content-type header value")?,
            );
        }
        headers.insert(
            reqwest::header::AUTHORIZATION,
            authorization
                .parse()
                .context("invalid authorization header value")?,
        );

        Ok(AliyunSignedRequest {
            query_string: canonical_query,
            headers,
        })
    }
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex_encode_lower(&hasher.finalize())
}

fn hmac_sha256_hex(secret: &str, message: &str) -> String {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize().into_bytes();
    hex_encode_lower(&result)
}

fn hex_encode_lower(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input {
        use std::fmt::Write;
        write!(&mut out, "{:02x}", b).expect("write into string");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v3_signature_example_from_docs() {
        // Example from Aliyun docs (V3 request structure & signature)
        // https://help.aliyun.com/zh/sdk/product-overview/v3-request-structure-and-signature
        let signer = AliyunSigner::new(
            "YourAccessKeyId".to_string(),
            "YourAccessKeySecret".to_string(),
        );

        // Build query params (these are the API request parameters in the docs example)
        let mut query_params = BTreeMap::new();
        query_params.insert(
            "ImageId".to_string(),
            "win2019_1809_x64_dtc_zh-cn_40G_alibase_20230811.vhd".to_string(),
        );
        query_params.insert("RegionId".to_string(), "cn-shanghai".to_string());

        // Keep deterministic verification without injecting timestamp/nonce into `sign_request`.
        let method = "POST";
        let host = "ecs.cn-shanghai.aliyuncs.com";
        let canonical_uri = "/";
        let action = "RunInstances";
        let version = "2014-05-26";
        let x_acs_date = "2023-10-26T10:22:32Z";
        let x_acs_signature_nonce = "3156853299f313e23d1673dc12e1703d";
        let x_acs_content_sha256 =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let canonical_query = AliyunSigner::build_canonical_query_string(&query_params);
        assert_eq!(
            canonical_query,
            "ImageId=win2019_1809_x64_dtc_zh-cn_40G_alibase_20230811.vhd&RegionId=cn-shanghai"
        );

        let mut signing_headers: BTreeMap<String, String> = BTreeMap::new();
        signing_headers.insert("host".to_string(), host.to_string());
        signing_headers.insert("x-acs-action".to_string(), action.to_string());
        signing_headers.insert(
            "x-acs-content-sha256".to_string(),
            x_acs_content_sha256.to_string(),
        );
        signing_headers.insert("x-acs-date".to_string(), x_acs_date.to_string());
        signing_headers.insert(
            "x-acs-signature-nonce".to_string(),
            x_acs_signature_nonce.to_string(),
        );
        signing_headers.insert("x-acs-version".to_string(), version.to_string());

        let canonical_headers = signing_headers
            .iter()
            .map(|(k, v)| format!("{}:{}\n", k, v))
            .collect::<String>();
        let signed_headers = signing_headers
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(";");

        assert_eq!(
            signed_headers,
            "host;x-acs-action;x-acs-content-sha256;x-acs-date;x-acs-signature-nonce;x-acs-version"
        );

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            AliyunSigner::canonicalize_uri(canonical_uri),
            canonical_query,
            canonical_headers,
            signed_headers,
            x_acs_content_sha256
        );

        let hashed_canonical_request = sha256_hex(canonical_request.as_bytes());
        assert_eq!(
            hashed_canonical_request,
            "7ea06492da5221eba5297e897ce16e55f964061054b7695beedaac1145b1e259"
        );

        let string_to_sign = format!("ACS3-HMAC-SHA256\n{}", hashed_canonical_request);
        let signature = hmac_sha256_hex(&signer.access_key_secret, &string_to_sign);
        assert_eq!(
            signature,
            "06563a9e1b43f5dfe96b81484da74bceab24a1d853912eee15083a6f0f3283c0"
        );
    }
}
