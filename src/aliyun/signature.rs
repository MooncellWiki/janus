use chrono::Utc;
use rand::Rng;
use sha2::Sha256;
use std::collections::BTreeMap;

/// Aliyun API V3 signature generator
/// 
/// Implements the Aliyun API signature V3 algorithm as documented at:
/// https://help.aliyun.com/zh/sdk/product-overview/v3-request-structure-and-signature
pub struct AliyunSigner {
    access_key_id: String,
    access_key_secret: String,
}

impl AliyunSigner {
    /// Create a new AliyunSigner with credentials
    pub fn new(access_key_id: String, access_key_secret: String) -> Self {
        Self {
            access_key_id,
            access_key_secret,
        }
    }

    /// Generate a random nonce for request
    fn generate_nonce() -> String {
        let mut rng = rand::thread_rng();
        let nonce: u64 = rng.r#gen();
        nonce.to_string()
    }

    /// Get current timestamp in ISO 8601 format (UTC)
    fn get_timestamp() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    /// Build canonical query string from parameters (sorted by key)
    fn build_canonical_query_string(params: &BTreeMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Sign a request with Aliyun V3 signature algorithm
    /// 
    /// # Arguments
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `params` - Request parameters (will be sorted)
    /// 
    /// # Returns
    /// A tuple of (signed_params, headers) where:
    /// - signed_params: Complete parameters including signature and common params
    /// - headers: HTTP headers to include in the request
    pub fn sign_request(
        &self,
        method: &str,
        mut params: BTreeMap<String, String>,
    ) -> (BTreeMap<String, String>, reqwest::header::HeaderMap) {
        // Add common parameters
        params.insert("AccessKeyId".to_string(), self.access_key_id.clone());
        params.insert("SignatureMethod".to_string(), "HMAC-SHA256".to_string());
        params.insert("SignatureVersion".to_string(), "1.0".to_string());
        params.insert("SignatureNonce".to_string(), Self::generate_nonce());
        params.insert("Timestamp".to_string(), Self::get_timestamp());
        params.insert("Format".to_string(), "JSON".to_string());

        // Build canonical query string (parameters are already sorted by BTreeMap)
        let canonical_query = Self::build_canonical_query_string(&params);

        // Build string to sign: METHOD&percent_encode(/)&percent_encode(canonical_query)
        let string_to_sign = format!(
            "{}&{}&{}",
            method.to_uppercase(),
            percent_encode("/"),
            percent_encode(&canonical_query)
        );

        // Calculate signature using HMAC-SHA256
        let signature = self.calculate_signature(&string_to_sign);

        // Add signature to parameters
        params.insert("Signature".to_string(), signature);

        // Build headers
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        (params, headers)
    }

    /// Calculate HMAC-SHA256 signature
    fn calculate_signature(&self, string_to_sign: &str) -> String {
        use hmac::{Hmac, Mac};
        use base64::Engine;
        type HmacSha256 = Hmac<Sha256>;

        // Key is AccessKeySecret + "&"
        let key = format!("{}&", self.access_key_secret);

        let mut mac = HmacSha256::new_from_slice(key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());

        // Get result and convert to base64
        let result = mac.finalize();
        base64::engine::general_purpose::STANDARD.encode(result.into_bytes())
    }
}

/// Percent encode a string according to RFC 3986
/// 
/// This encodes all characters except: A-Z, a-z, 0-9, -, _, ., ~
fn percent_encode(input: &str) -> String {
    input
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{:02X}", byte),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("/"), "%2F");
        assert_eq!(percent_encode("="), "%3D");
        assert_eq!(percent_encode("&"), "%26");
    }

    #[test]
    fn test_canonical_query_string() {
        let mut params = BTreeMap::new();
        params.insert("Action".to_string(), "DescribeRefreshTasks".to_string());
        params.insert("Version".to_string(), "2018-05-10".to_string());

        let query = AliyunSigner::build_canonical_query_string(&params);
        // BTreeMap sorts keys, so Action comes before Version
        assert!(query.contains("Action=DescribeRefreshTasks"));
        assert!(query.contains("Version=2018-05-10"));
    }
}
