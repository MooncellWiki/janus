use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::aliyun::{
    AliyunCdnClient, DescribeRefreshTasksRequest, DescribeRefreshTasksResponse,
    RefreshObjectCachesRequest, RefreshObjectCachesResponse,
};
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

/// Request payload for refresh object caches endpoint
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct RefreshObjectCachesPayload {
    /// Object paths to refresh (separated by newlines, max 1000 URLs or 100 directories per request)
    pub object_path: String,

    /// Object type: "File" for file refresh, "Directory" for directory refresh
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,

    /// Refresh area: "domestic" or "overseas"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
}

/// Refresh Aliyun CDN object caches
#[utoipa::path(
    post,
    tag = "aliyun",
    path = "/aliyun/refreshObjectCaches",
    request_body = RefreshObjectCachesPayload,
    responses(
        (status = OK, description = "Successfully submitted refresh task", body = RefreshObjectCachesResponse),
        (status = UNAUTHORIZED, description = "Unauthorized"),
        (status = BAD_REQUEST, description = "Invalid request parameters"),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn refresh_object_caches(
    State(state): State<AppState>,
    Json(payload): Json<RefreshObjectCachesPayload>,
) -> AppResult<Json<RefreshObjectCachesResponse>> {
    // Create Aliyun CDN client
    let client = AliyunCdnClient::new(&state.aliyun_config, state.http_client.clone());

    // Build request
    let request = RefreshObjectCachesRequest {
        object_path: payload.object_path,
        object_type: payload.object_type,
        area: payload.area,
    };

    // Call API
    let response = client.refresh_object_caches(&request).await?;

    Ok(Json(response))
}
