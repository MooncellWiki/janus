use axum::{Json, extract::State, http::HeaderMap};
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;

use crate::state::AppState;
use crate::{
    aliyun::{AliyunCdnClient, RefreshObjectCachesRequest},
    error::{AppError, AppResult},
};

/// OSS bucket information in event data
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct OssBucket {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arn: Option<String>,
    #[serde(rename = "ownerIdentity", skip_serializing_if = "Option::is_none")]
    pub owner_identity: Option<String>,
}

/// OSS object information in event data
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct OssObject {
    pub key: String,
    #[serde(rename = "eTag", skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(rename = "deltaSize", skip_serializing_if = "Option::is_none")]
    pub delta_size: Option<i64>,
}

/// OSS-specific data in event
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct OssData {
    pub bucket: OssBucket,
    pub object: OssObject,
    #[serde(rename = "ossSchemaVersion", skip_serializing_if = "Option::is_none")]
    pub oss_schema_version: Option<String>,
}

/// Complete event data structure from OSS
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct OssEventData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(rename = "eventVersion", skip_serializing_if = "Option::is_none")]
    pub event_version: Option<String>,
    #[serde(rename = "eventSource", skip_serializing_if = "Option::is_none")]
    pub event_source: Option<String>,
    #[serde(rename = "eventName", skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    #[serde(rename = "eventTime", skip_serializing_if = "Option::is_none")]
    pub event_time: Option<String>,
    pub oss: OssData,
}

/// EventBridge OSS event payload
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct OssEventPayload {
    pub id: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specversion: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacontenttype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    pub data: OssEventData,
}

/// Response for OSS event handler
#[derive(ToSchema, Serialize, Deserialize, Debug)]
pub struct OssEventResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Handle Aliyun EventBridge OSS events
#[utoipa::path(
    post,
    tag = "aliyun",
    path = "/aliyun/events",
    request_body = OssEventPayload,
    responses(
        (status = OK, description = "Successfully processed OSS event and triggered CDN refresh", body = OssEventResponse),
        (status = UNAUTHORIZED, description = "Missing or invalid x-eventbridge-signature-token"),
        (status = BAD_REQUEST, description = "Invalid request or unsupported bucket"),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error")
    ),
    security(
        ("eventbridge_token" = [])
    )
)]
pub async fn handle_oss_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(raw_payload): Json<serde_json::Value>,
) -> AppResult<Json<OssEventResponse>> {
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
                "Invalid x-eventbridge-signature-token header format"
            ))
        })?
        .trim();

    crate::auth::verify_token(token, &state.jwt_config.public_key).map_err(|err| {
        AppError::Unauthorized(anyhow::anyhow!(
            "JWT verification failed (x-eventbridge-signature-token): {err}"
        ))
    })?;

    // Parse the raw JSON into OssEventPayload
    let payload: OssEventPayload = serde_json::from_value(raw_payload).map_err(|err| {
        AppError::BadRequest(anyhow::anyhow!(
            "Failed to parse OSS event payload: {}",
            err
        ))
    })?;

    info!(
        event = ?payload,
        "Received OSS event"
    );

    let bucket_name = &payload.data.oss.bucket.name;
    let object_key = &payload.data.oss.object.key;

    // Get URL template from bucket map
    let url_template = state
        .aliyun_config
        .bucket_url_map
        .get(bucket_name)
        .ok_or_else(|| {
            AppError::BadRequest(anyhow::anyhow!("Unsupported bucket: {}", bucket_name))
        })?;

    // Build the full URL by replacing {object_key} with the actual encoded object key
    let encoded_object_key = percent_encode(object_key.as_bytes(), NON_ALPHANUMERIC).to_string();
    let object_url = url_template.replace("{object_key}", &encoded_object_key);

    // Create CDN client
    let client = AliyunCdnClient::new(&state.aliyun_config, state.http_client.clone());

    // Refresh the object cache
    let request = RefreshObjectCachesRequest {
        object_path: object_url.clone(),
        object_type: Some("File".to_string()),
        force: Some(false),
    };

    let response = client.refresh_object_caches(&request).await?;

    Ok(Json(OssEventResponse {
        message: format!(
            "CDN refresh triggered for {} in bucket {}",
            object_key, bucket_name
        ),
        task_id: Some(response.refresh_task_id),
    }))
}
