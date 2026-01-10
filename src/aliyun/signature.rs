use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Alibaba Cloud API V3 request signer
/// Implements HMAC-SHA256 signature algorithm for ACS3-HMAC-SHA256
#[derive(Debug, Clone)]
pub struct AliyunSigner {
    access_key_id: String,
    access_key_secret: String,
}

impl AliyunSigner {
    /// Create a new AliyunSigner
    pub fn new(access_key_id: String, access_key_secret: String) -> Self {
        Self {
            access_key_id,
            access_key_secret,
        }
    }

    /// Generate x-acs-date header value in ISO 8601 format
    /// Format: YYYY-MM-DDTHH:MM:SSZ
    fn generate_date() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    /// Generate a random nonce (UUID v4)
    fn generate_nonce() -> String {
        Uuid::new_v4().to_string()
    }

    /// Calculate SHA256 hash of request body and return as lowercase hex string
    fn hash_body(body: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Build canonical headers string
    /// Headers are sorted alphabetically and formatted as "header:value\n"
    fn build_canonical_headers(headers: &BTreeMap<String, String>) -> String {
        headers
            .iter()
            .map(|(k, v)| format!("{}:{}", k.to_lowercase(), v.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Build signed headers list (semicolon-separated, sorted alphabetically)
    fn build_signed_headers(headers: &BTreeMap<String, String>) -> String {
        headers
            .keys()
            .map(|k| k.to_lowercase())
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Build canonical query string
    /// Query parameters are sorted alphabetically and URL-encoded
    fn build_canonical_query(params: &BTreeMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Build the canonical request string for signing
    /// Format:
    /// HTTPMethod\n
    /// CanonicalURI\n
    /// CanonicalQueryString\n
    /// CanonicalHeaders\n
    /// SignedHeaders\n
    /// HashedPayload
    fn build_canonical_request(
        method: &str,
        uri: &str,
        query: &str,
        canonical_headers: &str,
        signed_headers: &str,
        hashed_payload: &str,
    ) -> String {
        format!(
            "{}\n{}\n{}\n{}\n\n{}\n{}",
            method, uri, query, canonical_headers, signed_headers, hashed_payload
        )
    }

    /// Calculate HMAC-SHA256 signature
    fn sign_string(string_to_sign: &str, secret: &str) -> String {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        format!("{:x}", mac.finalize().into_bytes())
    }

    /// Sign an API request and return the Authorization header value
    ///
    /// # Arguments
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `uri` - Request URI path (e.g., "/")
    /// * `query_params` - Query parameters as key-value pairs
    /// * `headers` - Additional headers to include in signature (host, x-acs-action, x-acs-version, etc.)
    /// * `body` - Request body (empty string for GET requests)
    ///
    /// # Returns
    /// A tuple of (authorization_header, x-acs-date, x-acs-signature-nonce, x-acs-content-sha256)
    pub fn sign_request(
        &self,
        method: &str,
        uri: &str,
        query_params: &BTreeMap<String, String>,
        mut headers: BTreeMap<String, String>,
        body: &str,
    ) -> (String, String, String, String) {
        // Generate timestamp and nonce
        let date = Self::generate_date();
        let nonce = Self::generate_nonce();
        let hashed_payload = Self::hash_body(body);

        // Add required headers to the signature headers
        headers.insert("x-acs-date".to_string(), date.clone());
        headers.insert("x-acs-signature-nonce".to_string(), nonce.clone());
        headers.insert("x-acs-content-sha256".to_string(), hashed_payload.clone());

        // Build canonical strings
        let canonical_headers = Self::build_canonical_headers(&headers);
        let signed_headers = Self::build_signed_headers(&headers);
        let canonical_query = Self::build_canonical_query(query_params);

        // Build canonical request
        let canonical_request = Self::build_canonical_request(
            method,
            uri,
            &canonical_query,
            &canonical_headers,
            &signed_headers,
            &hashed_payload,
        );

        // Hash the canonical request
        let hashed_canonical_request = Self::hash_body(&canonical_request);

        // Build string to sign
        let string_to_sign = format!("ACS3-HMAC-SHA256\n{}", hashed_canonical_request);

        // Calculate signature
        let signature = Self::sign_string(&string_to_sign, &self.access_key_secret);

        // Build authorization header
        let authorization = format!(
            "ACS3-HMAC-SHA256 Credential={},SignedHeaders={},Signature={}",
            self.access_key_id, signed_headers, signature
        );

        (authorization, date, nonce, hashed_payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_body() {
        let empty_hash = AliyunSigner::hash_body("");
        // SHA256 of empty string
        assert_eq!(
            empty_hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_build_canonical_headers() {
        let mut headers = BTreeMap::new();
        headers.insert("Host".to_string(), "cdn.aliyuncs.com".to_string());
        headers.insert(
            "x-acs-action".to_string(),
            "DescribeRefreshTasks".to_string(),
        );

        let canonical = AliyunSigner::build_canonical_headers(&headers);
        assert!(canonical.contains("host:cdn.aliyuncs.com"));
        assert!(canonical.contains("x-acs-action:DescribeRefreshTasks"));
    }

    #[test]
    fn test_build_signed_headers() {
        let mut headers = BTreeMap::new();
        headers.insert("Host".to_string(), "value".to_string());
        headers.insert("x-acs-action".to_string(), "value".to_string());

        let signed = AliyunSigner::build_signed_headers(&headers);
        assert_eq!(signed, "host;x-acs-action");
    }
}
