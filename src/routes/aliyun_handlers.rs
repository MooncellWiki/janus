use crate::aliyun::cdn::{
    AliyunCdnClient, DescribeRefreshTasksRequest, RefreshObjectCachesRequest,
};
use crate::auth::verify_token;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for describing refresh tasks
#[derive(Debug, Deserialize, ToSchema)]
pub struct DescribeRefreshTasksRequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

/// Request body for refreshing object caches
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshObjectCachesRequestBody {
    pub object_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
}

/// Generic success response
#[derive(Debug, Serialize, ToSchema)]
pub struct SuccessResponse {
    pub code: i32,
}

/// Describe CDN refresh tasks
#[utoipa::path(
    post,
    path = "/api/aliyun/describeRefreshTasks",
    tag = "aliyun",
    request_body = DescribeRefreshTasksRequestBody,
    responses(
        (status = 200, description = "Refresh tasks retrieved successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = SuccessResponse),
        (status = 500, description = "Internal server error", body = SuccessResponse),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn describe_refresh_tasks(
    State(state): State<AppState>,
    Json(req): Json<DescribeRefreshTasksRequestBody>,
) -> AppResult<Json<serde_json::Value>> {
    let aliyun_config = state.aliyun_config.ok_or_else(|| {
        AppError::InternalError(anyhow::anyhow!("Aliyun configuration not found"))
    })?;

    let client = AliyunCdnClient::new(
        aliyun_config.access_key_id,
        aliyun_config.access_key_secret,
        state.http_client,
    );

    let request = DescribeRefreshTasksRequest {
        domain_name: req.domain_name,
        task_id: req.task_id,
        object_path: req.object_path,
        page_number: req.page_number,
        page_size: req.page_size,
        object_type: req.object_type,
        status: req.status,
        start_time: req.start_time,
        end_time: req.end_time,
    };

    let response = client
        .describe_refresh_tasks(&request)
        .await
        .map_err(AppError::InternalError)?;

    Ok(Json(serde_json::to_value(response)?))
}

/// Refresh CDN object caches
#[utoipa::path(
    post,
    path = "/api/aliyun/refreshObjectCaches",
    tag = "aliyun",
    request_body = RefreshObjectCachesRequestBody,
    responses(
        (status = 200, description = "Cache refresh initiated successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized", body = SuccessResponse),
        (status = 500, description = "Internal server error", body = SuccessResponse),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn refresh_object_caches(
    State(state): State<AppState>,
    Json(req): Json<RefreshObjectCachesRequestBody>,
) -> AppResult<Json<serde_json::Value>> {
    let aliyun_config = state.aliyun_config.ok_or_else(|| {
        AppError::InternalError(anyhow::anyhow!("Aliyun configuration not found"))
    })?;

    let client = AliyunCdnClient::new(
        aliyun_config.access_key_id,
        aliyun_config.access_key_secret,
        state.http_client,
    );

    let request = RefreshObjectCachesRequest {
        object_path: req.object_path,
        object_type: req.object_type,
        area: req.area,
    };

    let response = client
        .refresh_object_caches(&request)
        .await
        .map_err(AppError::InternalError)?;

    Ok(Json(serde_json::to_value(response)?))
}

/// OSS EventBridge event structures
#[derive(Debug, Deserialize, ToSchema)]
pub struct OssEventPayload {
    pub data: OssEventData,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OssEventData {
    pub oss: OssData,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OssData {
    pub bucket: OssBucket,
    pub object: OssObject,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OssBucket {
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OssObject {
    pub key: String,
}

/// Handle OSS EventBridge events and trigger CDN refresh
#[utoipa::path(
    post,
    path = "/api/aliyun/events",
    tag = "aliyun",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Event processed successfully", body = SuccessResponse),
        (status = 401, description = "Unauthorized", body = SuccessResponse),
        (status = 500, description = "Internal server error", body = SuccessResponse),
    ),
    security(
        ("eventbridge_token" = [])
    )
)]
pub async fn handle_oss_events(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> AppResult<Json<SuccessResponse>> {
    // Extract and verify EventBridge signature token
    let headers = req.headers();
    let token = headers
        .get("x-eventbridge-signature-token")
        .ok_or_else(|| {
            AppError::Unauthorized(anyhow::anyhow!(
                "Missing x-eventbridge-signature-token header"
            ))
        })?
        .to_str()
        .map_err(|_| {
            AppError::Unauthorized(anyhow::anyhow!(
                "Invalid x-eventbridge-signature-token format"
            ))
        })?;

    // Verify JWT token
    verify_token(token, &state.jwt_config.public_key).map_err(|err| {
        AppError::Unauthorized(anyhow::anyhow!(
            "EventBridge token verification failed: {}",
            err
        ))
    })?;

    // Read request body
    let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to read request body: {}", e))
        })?;

    let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Invalid UTF-8 in request body: {}", e))
    })?;

    // Log raw payload for debugging
    tracing::debug!("Received OSS event payload: {}", body_str);

    // Parse event
    let event: OssEventPayload = serde_json::from_str(&body_str).map_err(|e| {
        AppError::BadRequest(anyhow::anyhow!("Failed to parse event payload: {}", e))
    })?;

    let bucket_name = &event.data.oss.bucket.name;
    let object_key = &event.data.oss.object.key;

    tracing::info!(
        "Processing OSS event for bucket: {}, object: {}",
        bucket_name,
        object_key
    );

    // Get Aliyun config
    let aliyun_config = state.aliyun_config.ok_or_else(|| {
        AppError::InternalError(anyhow::anyhow!("Aliyun configuration not found"))
    })?;

    // Look up URL template for bucket
    let url_template = aliyun_config
        .bucket_url_map
        .get(bucket_name)
        .ok_or_else(|| {
            AppError::BadRequest(anyhow::anyhow!(
                "No URL mapping found for bucket: {}",
                bucket_name
            ))
        })?;

    // Build URL by replacing {object_key} placeholder with percent-encoded object key
    let encoded_key = urlencoding::encode(object_key);
    let cdn_url = url_template.replace("{object_key}", &encoded_key);

    tracing::info!("Triggering CDN refresh for URL: {}", cdn_url);

    // Create CDN client and refresh cache
    let cdn_client = AliyunCdnClient::new(
        aliyun_config.access_key_id.clone(),
        aliyun_config.access_key_secret.clone(),
        state.http_client.clone(),
    );

    let refresh_request = RefreshObjectCachesRequest {
        object_path: cdn_url.clone(),
        object_type: Some("File".to_string()),
        area: None,
    };

    cdn_client
        .refresh_object_caches(&refresh_request)
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to refresh CDN cache: {}", e))
        })?;

    tracing::info!("Successfully triggered CDN refresh for: {}", cdn_url);

    Ok(Json(SuccessResponse { code: 0 }))
}
