use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::aliyun::{AliyunCdnClient, DescribeRefreshTasksRequest, DescribeRefreshTasksResponse};
use crate::error::AppResult;
use crate::state::AppState;

/// Request payload for describe refresh tasks endpoint
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct DescribeRefreshTasksPayload {
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

/// Query Aliyun CDN refresh tasks
#[utoipa::path(
    post,
    tag = "aliyun",
    path = "/aliyun/describeRefreshTasks",
    request_body = DescribeRefreshTasksPayload,
    responses(
        (status = OK, description = "Successfully retrieved refresh tasks", body = DescribeRefreshTasksResponse),
        (status = UNAUTHORIZED, description = "Unauthorized"),
        (status = BAD_REQUEST, description = "Invalid request parameters"),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn describe_refresh_tasks(
    State(state): State<AppState>,
    Json(payload): Json<DescribeRefreshTasksPayload>,
) -> AppResult<Json<DescribeRefreshTasksResponse>> {
    // Create Aliyun CDN client
    let client = AliyunCdnClient::new(&state.aliyun_config, state.http_client.clone());

    // Build request
    let request = DescribeRefreshTasksRequest {
        task_id: payload.task_id,
        object_path: payload.object_path,
        page_number: payload.page_number,
        page_size: payload.page_size,
        object_type: payload.object_type,
        domain_name: payload.domain_name,
        status: payload.status,
        start_time: payload.start_time,
        end_time: payload.end_time,
    };

    // Call API
    let response = client.describe_refresh_tasks(&request).await?;

    Ok(Json(response))
}
