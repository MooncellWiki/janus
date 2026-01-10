use super::signature::AliyunSigner;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

/// CDN API client for Alibaba Cloud
#[derive(Debug, Clone)]
pub struct CdnClient {
    signer: AliyunSigner,
    endpoint: String,
    http_client: reqwest::Client,
}

/// Parameters for DescribeRefreshTasks API
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeRefreshTasksParams {
    /// The ID of the refresh or prefetch task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// The path (URL) of the object for an exact match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_path: Option<String>,
    /// The page number for paginated results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i32>,
    /// The type of the task (file, directory, preload, regex, block, unblock)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    /// The accelerated domain name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_name: Option<String>,
    /// Task status (Complete, Refreshing, Failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Number of entries per page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    /// The beginning of the time range for query (ISO8601 UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// The end of the time range for query (ISO8601 UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// The resource group ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_group_id: Option<String>,
}

/// Response from DescribeRefreshTasks API
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct DescribeRefreshTasksResponse {
    /// Request ID
    pub request_id: String,
    /// Total count of tasks
    pub total_count: i64,
    /// Page number
    pub page_number: i64,
    /// Page size
    pub page_size: i64,
    /// List of tasks
    #[serde(default)]
    pub tasks: TasksWrapper,
}

/// Wrapper for tasks array
#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct TasksWrapper {
    #[serde(default)]
    pub c_d_n_task: Vec<RefreshTask>,
}

/// Individual refresh task
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct RefreshTask {
    /// Task ID
    pub task_id: String,
    /// Object path
    pub object_path: String,
    /// Process percentage
    pub process: String,
    /// Task status
    pub status: String,
    /// Creation time
    pub creation_time: String,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Object type
    pub object_type: String,
}

impl CdnClient {
    /// Create a new CDN client
    pub fn new(access_key_id: String, access_key_secret: String, endpoint: String) -> Self {
        Self {
            signer: AliyunSigner::new(access_key_id, access_key_secret),
            endpoint,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create CDN client with custom HTTP client (useful for sharing client across app)
    pub fn with_client(
        access_key_id: String,
        access_key_secret: String,
        endpoint: String,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            signer: AliyunSigner::new(access_key_id, access_key_secret),
            endpoint,
            http_client,
        }
    }

    /// Call DescribeRefreshTasks API to query refresh task status
    ///
    /// # Arguments
    /// * `params` - Query parameters for the API call
    ///
    /// # Returns
    /// Result containing the API response or an error
    pub async fn describe_refresh_tasks(
        &self,
        params: DescribeRefreshTasksParams,
    ) -> Result<DescribeRefreshTasksResponse, anyhow::Error> {
        let method = "POST";
        let uri = "/";
        let api_version = "2018-05-10";

        // Build query params - empty for POST requests
        let query_params = BTreeMap::new();

        // Build request headers for signature
        let mut headers = BTreeMap::new();
        headers.insert("host".to_string(), self.endpoint.clone());
        headers.insert(
            "x-acs-action".to_string(),
            "DescribeRefreshTasks".to_string(),
        );
        headers.insert("x-acs-version".to_string(), api_version.to_string());

        // Serialize request body
        let body = serde_json::to_string(&params)?;

        // Sign the request
        let (authorization, date, nonce, content_sha256) =
            self.signer
                .sign_request(method, uri, &query_params, headers, &body);

        // Build HTTP headers
        let mut req_headers = reqwest::header::HeaderMap::new();
        req_headers.insert("host", self.endpoint.parse()?);
        req_headers.insert("x-acs-action", "DescribeRefreshTasks".parse()?);
        req_headers.insert("x-acs-version", api_version.parse()?);
        req_headers.insert("x-acs-date", date.parse()?);
        req_headers.insert("x-acs-signature-nonce", nonce.parse()?);
        req_headers.insert("x-acs-content-sha256", content_sha256.parse()?);
        req_headers.insert("authorization", authorization.parse()?);
        req_headers.insert("content-type", "application/json".parse()?);

        // Send request
        let url = format!("https://{}{}", self.endpoint, uri);
        let response = self
            .http_client
            .post(&url)
            .headers(req_headers)
            .body(body)
            .send()
            .await?;

        // Check response status
        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "API request failed with status {}: {}",
                status,
                response_text
            ));
        }

        // Parse response
        let result: DescribeRefreshTasksResponse = serde_json::from_str(&response_text)?;
        Ok(result)
    }
}
