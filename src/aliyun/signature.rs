use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

/// Aliyun API V3 signature generator
///
/// Implements the signature algorithm for Aliyun API V3 as documented at:
/// https://help.aliyun.com/zh/sdk/product-overview/v3-request-structure-and-signature
pub struct AliyunSigner {
    access_key_id: String,
    access_key_secret: String,
}

impl AliyunSigner {
    /// Create a new AliyunSigner with access credentials
    pub fn new(access_key_id: String, access_key_secret: String) -> Self {
        Self {
            access_key_id,
            access_key_secret,
        }
    }

    /// Generate signature for API request
    ///
    /// # Arguments
    /// * `method` - HTTP method (e.g., "GET", "POST")
    /// * `path` - API path (e.g., "/")
    /// * `params` - Query parameters as key-value pairs
    ///
    /// # Returns
    /// Returns a tuple of (query_string, signature) where:
    /// - query_string: URL-encoded query string with all parameters
    /// - signature: Base64-encoded HMAC-SHA256 signature
    pub fn sign(
        &self,
        method: &str,
        path: &str,
        params: &BTreeMap<String, String>,
    ) -> (String, String) {
        // Build canonical query string from sorted parameters
        let canonical_query = self.build_canonical_query(params);

        // Build string to sign
        let string_to_sign = format!("{}\n{}\n{}", method, path, canonical_query);

        // Generate signature
        let signature_key = format!("{}&", self.access_key_secret);
        let mut mac = HmacSha256::new_from_slice(signature_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let signature_bytes = mac.finalize().into_bytes();
        let signature = general_purpose::STANDARD.encode(signature_bytes);

        (canonical_query, signature)
    }

    /// Build canonical query string from parameters
    /// Parameters are sorted by key (BTreeMap maintains order) and percent-encoded per RFC 3986
    fn build_canonical_query(&self, params: &BTreeMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Get access key ID
    pub fn access_key_id(&self) -> &str {
        &self.access_key_id
    }
}

/// Percent-encode a string according to RFC 3986
///
/// This encodes all characters except: A-Z, a-z, 0-9, -, _, ., ~
/// Space is encoded as %20 (not +)
fn percent_encode(s: &str) -> String {
    urlencoding::encode(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("test@example.com"), "test%40example.com");
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn test_signature_generation() {
        let signer = AliyunSigner::new("test_key_id".to_string(), "test_key_secret".to_string());

        let mut params = BTreeMap::new();
        params.insert("Action".to_string(), "DescribeRefreshTasks".to_string());
        params.insert("Version".to_string(), "2018-05-10".to_string());

        let (query, signature) = signer.sign("GET", "/", &params);

        // Verify query string is properly formatted
        assert!(query.contains("Action=DescribeRefreshTasks"));
        assert!(query.contains("Version=2018-05-10"));

        // Verify signature is base64 encoded
        assert!(general_purpose::STANDARD.decode(&signature).is_ok());
    }
}
