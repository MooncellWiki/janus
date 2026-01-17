use crate::aliyun::cdn::{AliyunCdnClient, RefreshObjectCachesRequest};
use crate::auth::verify_token;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Generic success response
#[derive(Debug, Serialize, ToSchema)]
pub struct SuccessResponse {
    pub code: i32,
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
