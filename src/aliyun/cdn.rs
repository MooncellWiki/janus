use crate::aliyun::signature::AliyunSigner;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const CDN_ENDPOINT: &str = "https://cdn.aliyuncs.com";
const API_VERSION: &str = "2018-05-10";

/// Aliyun CDN API client
pub struct AliyunCdnClient {
    signer: AliyunSigner,
    http_client: reqwest::Client,
}

impl AliyunCdnClient {
    /// Create a new CDN client
    pub fn new(
        access_key_id: String,
        access_key_secret: String,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            signer: AliyunSigner::new(access_key_id, access_key_secret),
            http_client,
        }
    }

    /// Refresh CDN object caches
    ///
    /// API documentation: https://help.aliyun.com/zh/cdn/developer-reference/api-cdn-2018-05-10-refreshobjectcaches
    pub async fn refresh_object_caches(
        &self,
        request: &RefreshObjectCachesRequest,
    ) -> Result<RefreshObjectCachesResponse> {
        let mut params = BTreeMap::new();

        // Required parameters
        params.insert("Action".to_string(), "RefreshObjectCaches".to_string());
        params.insert("Version".to_string(), API_VERSION.to_string());
        params.insert("Format".to_string(), "JSON".to_string());
        params.insert(
            "AccessKeyId".to_string(),
            self.signer.access_key_id().to_string(),
        );
        params.insert("SignatureMethod".to_string(), "HMAC-SHA256".to_string());
        params.insert("SignatureVersion".to_string(), "1.0".to_string());
        params.insert(
            "SignatureNonce".to_string(),
            uuid::Uuid::new_v4().to_string(),
        );
        params.insert(
            "Timestamp".to_string(),
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        );
        params.insert("ObjectPath".to_string(), request.object_path.clone());

        // Optional parameters
        if let Some(object_type) = &request.object_type {
            params.insert("ObjectType".to_string(), object_type.clone());
        }
        if let Some(area) = &request.area {
            params.insert("Area".to_string(), area.clone());
        }

        // Generate signature
        let (query_string, signature) = self.signer.sign("GET", "/", &params);

        // Build final URL (parameters are already percent-encoded in query_string)
        let url = format!(
            "{}/?{}&Signature={}",
            CDN_ENDPOINT,
            query_string,
            urlencoding::encode(&signature)
        );

        // Make request
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to Aliyun CDN API")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            anyhow::bail!("Aliyun CDN API error ({}): {}", status, body);
        }

        serde_json::from_str(&body).context("Failed to parse Aliyun CDN API response")
    }
}

// Request/Response structures

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshObjectCachesRequest {
    pub object_path: String,
    pub object_type: Option<String>,
    pub area: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefreshObjectCachesResponse {
    pub request_id: String,
    pub refresh_task_id: String,
}
