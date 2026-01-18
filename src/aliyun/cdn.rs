use crate::config::AliyunConfig;
use crate::error::{AppError, AppResult};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

use super::signature::{AliyunSignInput, AliyunSigner};

/// CDN API endpoint
const CDN_ENDPOINT: &str = "https://cdn.aliyuncs.com";
const CDN_HOST: &str = "cdn.aliyuncs.com";

/// Request parameters for DescribeRefreshTasks API
///
/// Reference: https://help.aliyun.com/zh/cdn/developer-reference/api-cdn-2018-05-10-describerefreshtasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeRefreshTasksRequest {
    /// Task ID for querying specific task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Object path for filtering tasks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_path: Option<String>,

    /// Page number (starting from 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i32>,

    /// Page size (default 20, max 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,

    /// Task type filter: "file" or "directory"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,

    /// Domain name filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_name: Option<String>,

    /// Status filter: "Complete", "Refreshing", "Failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Start time (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,

    /// End time (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

/// Response from DescribeRefreshTasks API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DescribeRefreshTasksResponse {
    #[serde(rename = "RequestId")]
    pub request_id: String,

    #[serde(rename = "PageNumber")]
    pub page_number: i64,

    #[serde(rename = "PageSize")]
    pub page_size: i64,

    #[serde(rename = "TotalCount")]
    pub total_count: i64,

    #[serde(rename = "Tasks")]
    pub tasks: TasksContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TasksContainer {
    #[serde(rename = "CDNTask")]
    pub cdn_tasks: Vec<RefreshTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTask {
    #[serde(rename = "TaskId")]
    pub task_id: String,

    #[serde(rename = "ObjectPath")]
    pub object_path: String,

    #[serde(rename = "ObjectType")]
    pub object_type: String,

    #[serde(rename = "Status")]
    pub status: String,

    #[serde(rename = "Process")]
    pub process: String,

    #[serde(rename = "CreationTime")]
    pub creation_time: String,

    #[serde(rename = "Description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request parameters for RefreshObjectCaches API
///
/// Reference: https://help.aliyun.com/zh/cdn/developer-reference/api-cdn-2018-05-10-refreshobjectcaches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshObjectCachesRequest {
    /// Object paths to refresh (separated by newlines, max 1000 URLs or 100 directories per request)
    pub object_path: String,

    /// Object type: "File" for file refresh, "Directory" for directory refresh
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,

    /// Whether to directly delete CDN cache nodes (default false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

/// Form parameters for RefreshObjectCaches API
/// This struct is used for URL encoding the request body
#[derive(Debug, Clone, Serialize)]
struct RefreshObjectCachesFormParams {
    #[serde(rename = "ObjectPath")]
    object_path: String,

    #[serde(rename = "ObjectType", skip_serializing_if = "Option::is_none")]
    object_type: Option<String>,

    #[serde(rename = "Force", skip_serializing_if = "Option::is_none")]
    force: Option<bool>,
}

/// Response from RefreshObjectCaches API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshObjectCachesResponse {
    #[serde(rename = "RequestId")]
    pub request_id: String,

    #[serde(rename = "RefreshTaskId")]
    pub refresh_task_id: String,
}

/// Aliyun CDN API client
pub struct AliyunCdnClient {
    signer: AliyunSigner,
    client: reqwest::Client,
}

impl AliyunCdnClient {
    /// Create a new Aliyun CDN client
    pub fn new(config: &AliyunConfig, client: reqwest::Client) -> Self {
        let signer = AliyunSigner::new(
            config.access_key_id.clone(),
            config.access_key_secret.clone(),
        );

        Self { signer, client }
    }

    /// Call DescribeRefreshTasks API
    ///
    /// # Arguments
    /// * `request` - Request parameters
    ///
    /// # Returns
    /// Response containing refresh task information
    pub async fn describe_refresh_tasks(
        &self,
        request: &DescribeRefreshTasksRequest,
    ) -> AppResult<DescribeRefreshTasksResponse> {
        // Build query parameters (V3: Action/Version are sent as x-acs-* headers)
        let mut params = BTreeMap::new();

        // Add optional parameters
        if let Some(ref task_id) = request.task_id {
            params.insert("TaskId".to_string(), task_id.clone());
        }
        if let Some(ref object_path) = request.object_path {
            params.insert("ObjectPath".to_string(), object_path.clone());
        }
        if let Some(page_number) = request.page_number {
            params.insert("PageNumber".to_string(), page_number.to_string());
        }
        if let Some(page_size) = request.page_size {
            params.insert("PageSize".to_string(), page_size.to_string());
        }
        if let Some(ref object_type) = request.object_type {
            params.insert("ObjectType".to_string(), object_type.clone());
        }
        if let Some(ref domain_name) = request.domain_name {
            params.insert("DomainName".to_string(), domain_name.clone());
        }
        if let Some(ref status) = request.status {
            params.insert("Status".to_string(), status.clone());
        }
        if let Some(ref start_time) = request.start_time {
            params.insert("StartTime".to_string(), start_time.clone());
        }
        if let Some(ref end_time) = request.end_time {
            params.insert("EndTime".to_string(), end_time.clone());
        }

        // Sign the request (ACS3-HMAC-SHA256)
        let signed = self
            .signer
            .sign_request(AliyunSignInput {
                method: "GET",
                host: CDN_HOST,
                canonical_uri: "/",
                action: "DescribeRefreshTasks",
                version: "2018-05-10",
                query_params: params,
                body: b"",
                content_type: None,
                extra_headers: BTreeMap::new(),
            })
            .context("Failed to sign Aliyun request")?;

        let query_string = signed.query_string;
        let headers = signed.headers;

        let url = if query_string.is_empty() {
            format!("{}/", CDN_ENDPOINT)
        } else {
            format!("{}/?{}", CDN_ENDPOINT, query_string)
        };

        // Send request
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("Failed to send DescribeRefreshTasks request")?;

        // Parse response
        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            return Err(AppError::InternalError(anyhow::anyhow!(
                "Aliyun API error (status {}): {}",
                status,
                body
            )));
        }
        // Parse JSON response
        let result: DescribeRefreshTasksResponse =
            serde_json::from_str(&body).context("Failed to parse DescribeRefreshTasks response")?;

        Ok(result)
    }

    /// Call RefreshObjectCaches API
    ///
    /// # Arguments
    /// * `request` - Request parameters
    ///
    /// # Returns
    /// Response containing refresh task ID
    pub async fn refresh_object_caches(
        &self,
        request: &RefreshObjectCachesRequest,
    ) -> AppResult<RefreshObjectCachesResponse> {
        // RefreshObjectCaches is a POST request with parameters in an HTML form body.
        // Reference: https://help.aliyun.com/zh/cdn/developer-reference/api-cdn-2018-05-10-refreshobjectcaches
        let form_params = RefreshObjectCachesFormParams {
            object_path: request.object_path.clone(),
            object_type: request.object_type.clone(),
            force: request.force,
        };

        let form_body = serde_urlencoded::to_string(&form_params)
            .context("Failed to encode form parameters")?;

        // Sign the request (ACS3-HMAC-SHA256). For this API, the form body must be included
        // in the body hash, so keep the canonical query empty.
        let signed = self
            .signer
            .sign_request(AliyunSignInput {
                method: "POST",
                host: CDN_HOST,
                canonical_uri: "/",
                action: "RefreshObjectCaches",
                version: "2018-05-10",
                query_params: BTreeMap::new(),
                body: form_body.as_bytes(),
                content_type: Some("application/x-www-form-urlencoded"),
                extra_headers: BTreeMap::new(),
            })
            .context("Failed to sign Aliyun request")?;

        let query_string = signed.query_string;
        let headers = signed.headers;

        let url = if query_string.is_empty() {
            format!("{}/", CDN_ENDPOINT)
        } else {
            format!("{}/?{}", CDN_ENDPOINT, query_string)
        };

        // Send request
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .body(form_body)
            .send()
            .await
            .context("Failed to send RefreshObjectCaches request")?;

        // Parse response
        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            return Err(AppError::InternalError(anyhow::anyhow!(
                "Aliyun API error (status {}): {}",
                status,
                body
            )));
        }

        // Parse JSON response
        let result: RefreshObjectCachesResponse =
            serde_json::from_str(&body).context("Failed to parse RefreshObjectCaches response")?;

        Ok(result)
    }
}
