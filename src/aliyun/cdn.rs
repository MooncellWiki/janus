use crate::config::AliyunConfig;
use crate::error::{AppError, AppResult};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

use super::signature::AliyunSigner;

/// CDN API endpoint
const CDN_ENDPOINT: &str = "https://cdn.aliyuncs.com";

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
    #[serde(rename = "Task")]
    pub task: Vec<RefreshTask>,
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
        // Build parameters
        let mut params = BTreeMap::new();
        params.insert("Action".to_string(), "DescribeRefreshTasks".to_string());
        params.insert("Version".to_string(), "2018-05-10".to_string());

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

        // Sign the request
        let (signed_params, headers) = self.signer.sign_request("GET", params);

        // Build query string (parameters already encoded during signing)
        let query_string = signed_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("{}/?{}", CDN_ENDPOINT, query_string);

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
}
