use axum::{Json, debug_handler, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    aliyun::cdn::{DescribeRefreshTasksParams, DescribeRefreshTasksResponse},
    error::{AppError, AppResult},
    state::AppState,
};

/// Request body for DescribeRefreshTasks endpoint
#[derive(ToSchema, Serialize, Deserialize, Default)]
pub struct DescribeRefreshTasksRequest {
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
    /// The beginning of the time range for query (ISO8601 UTC, e.g., "2023-12-21T08:00:00Z")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// The end of the time range for query (ISO8601 UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// The resource group ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_group_id: Option<String>,
}

/// Response wrapper for API responses
#[derive(ToSchema, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

/// Query Alibaba Cloud CDN refresh task status
#[debug_handler]
#[utoipa::path(
    post,
    tag = "aliyun",
    path = "/aliyun/cdn/describeRefreshTasks",
    request_body = DescribeRefreshTasksRequest,
    responses(
        (status = OK, description = "Successfully retrieved refresh tasks", body = ApiResponse<DescribeRefreshTasksResponse>),
        (status = UNAUTHORIZED, description = "Unauthorized"),
        (status = BAD_REQUEST, description = "Bad request"),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn describe_refresh_tasks(
    State(state): State<AppState>,
    Json(request): Json<DescribeRefreshTasksRequest>,
) -> AppResult<Json<ApiResponse<DescribeRefreshTasksResponse>>> {
    // Check if Alibaba Cloud client is configured
    let cdn_client = state
        .aliyun_client
        .as_ref()
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("Alibaba Cloud is not configured")))?;

    // Convert request to API params
    let params = DescribeRefreshTasksParams {
        task_id: request.task_id,
        object_path: request.object_path,
        page_number: request.page_number,
        object_type: request.object_type,
        domain_name: request.domain_name,
        status: request.status,
        page_size: request.page_size,
        start_time: request.start_time,
        end_time: request.end_time,
        resource_group_id: request.resource_group_id,
    };

    // Call CDN API
    let result = cdn_client
        .describe_refresh_tasks(params)
        .await
        .map_err(AppError::InternalError)?;

    Ok(Json(ApiResponse {
        code: 0,
        msg: None,
        data: Some(result),
    }))
}
