use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::AppState;
use crate::{
    aliyun::{
        AliyunCdnClient, DescribeRefreshTasksRequest, DescribeRefreshTasksResponse,
        RefreshObjectCachesRequest, RefreshObjectCachesResponse,
    },
    error::{AppError, AppResult},
};

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

    /// Whether to directly delete CDN cache nodes (default false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
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
        force: payload.force,
    };

    // Call API
    let response = client.refresh_object_caches(&request).await?;

    Ok(Json(response))
}

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

/// Map OSS bucket name to CDN domain
fn map_bucket_to_domain(bucket_name: &str) -> Option<String> {
    match bucket_name {
        "prts-static" => Some("static.prts.wiki".to_string()),
        "ak-media" => Some("media.prts.wiki".to_string()),
        _ => None,
    }
}

/// Percent-encode a path for use in URLs (RFC 3986)
/// Encodes all characters except unreserved characters (A-Z, a-z, 0-9, -, _, ., ~) and forward slash
fn percent_encode_path(input: &str) -> String {
    input
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                (byte as char).to_string()
            }
            _ => format!("%{:02X}", byte),
        })
        .collect()
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
    // Print the entire received JSON for debugging (debug level to avoid exposing sensitive data)
    tracing::debug!(
        "Received OSS event: {}",
        serde_json::to_string_pretty(&raw_payload)
            .unwrap_or_else(|_| "<invalid JSON payload>".to_string())
    );

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
        AppError::BadRequest(anyhow::anyhow!("Failed to parse OSS event payload: {}", err))
    })?;

    let bucket_name = &payload.data.oss.bucket.name;
    let object_key = &payload.data.oss.object.key;

    // Map bucket to CDN domain
    let domain = map_bucket_to_domain(bucket_name).ok_or_else(|| {
        AppError::BadRequest(anyhow::anyhow!("Unsupported bucket: {}", bucket_name))
    })?;

    // Build the full URL for the object with proper URL encoding
    let encoded_object_key = percent_encode_path(object_key);
    let object_url = format!("https://{}/{}", domain, encoded_object_key);

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
            "CDN refresh triggered for {} on domain {}",
            object_key, domain
        ),
        task_id: Some(response.refresh_task_id),
    }))
}
