use anyhow::Context;
use axum::{
    Json, debug_handler,
    extract::{Multipart, State},
};
use rand::Rng;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Response for createDynamic endpoint
#[derive(ToSchema, Serialize, Deserialize)]
pub struct DynamicResponse {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception: Option<serde_json::Value>,
}

/// Bilibili upload response
#[derive(Debug, Deserialize)]
struct BilibiliUploadResponse {
    code: i32,
    data: Option<BilibiliUploadData>,
}

#[derive(Debug, Deserialize)]
struct BilibiliUploadData {
    image_url: String,
    image_width: f64,
    image_height: f64,
}

/// Bilibili create dynamic response
#[derive(Debug, Deserialize, Serialize)]
struct BilibiliCreateResponse {
    code: i32,
    data: Option<BilibiliCreateData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct BilibiliCreateData {
    #[serde(default)]
    doc_id: Option<u64>,
    #[serde(default)]
    dynamic_id: Option<u64>,
    #[serde(default)]
    create_result: Option<i32>,
    #[serde(default)]
    errmsg: Option<String>,
}

/// Picture info for dynamic request
#[derive(Debug, Serialize)]
struct PicInfo {
    img_src: String,
    img_width: f64,
    img_height: f64,
    img_size: f64,
}

/// Generate headers for Bilibili API requests
fn create_headers(sessdata: &str) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "*/*".parse().unwrap());
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36"
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Sec-Ch-Ua",
        "\"Not A(Brand\";v=\"99\", \"Google Chrome\";v=\"121\", \"Chromium\";v=\"121\""
            .parse()
            .unwrap(),
    );
    headers.insert("Sec-Ch-Ua-Mobile", "?0".parse().unwrap());
    headers.insert("Sec-Ch-Ua-Platform", "\"Windows\"".parse().unwrap());
    headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
    headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
    headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());
    headers.insert(
        "Cookie",
        format!("SESSDATA={}; l=v", sessdata).parse().unwrap(),
    );
    headers
}

/// Generate random nonce
fn get_nonce() -> i32 {
    rand::thread_rng().gen_range(1000..9999)
}

/// Get unix timestamp in seconds
fn get_unix_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time should be after UNIX epoch")
        .as_secs_f64()
}

/// Upload a single image to Bilibili
async fn upload_image(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
    sessdata: &str,
    bili_jct: &str,
    client: &reqwest::Client,
) -> AppResult<(f64, BilibiliUploadData)> {
    let file_size_kb = file_data.len() as f64 / 1024.0;

    let file_part = Part::bytes(file_data)
        .file_name(file_name)
        .mime_str(&content_type)
        .map_err(|e| {
            AppError::InternalError(anyhow::Error::new(e).context("Failed to create file part"))
        })?;

    let form = Form::new()
        .part("file_up", file_part)
        .text("biz", "draw")
        .text("category", "daily")
        .text("csrf", bili_jct.to_string());

    let resp = client
        .post("https://api.bilibili.com/x/dynamic/feed/draw/upload_bfs")
        .headers(create_headers(sessdata))
        .multipart(form)
        .send()
        .await
        .context("Upload request failed")?;

    let resp_text = resp.text().await.context("Failed to read response")?;

    let upload_resp: BilibiliUploadResponse =
        serde_json::from_str(&resp_text).context("Failed to parse upload response")?;

    if upload_resp.code != 0 {
        return Err(AppError::InternalError(anyhow::anyhow!(
            "Bilibili file upload failed, response: {}",
            resp_text
        )));
    }

    let data = upload_resp
        .data
        .ok_or_else(|| AppError::InternalError(anyhow::anyhow!("Upload response missing data")))?;

    Ok((file_size_kb, data))
}

/// Helper function to handle Bilibili create dynamic response
async fn handle_create_dynamic_response(
    result: Result<reqwest::Response, reqwest::Error>,
) -> AppResult<serde_json::Value> {
    let resp = result.context("Create dynamic request failed")?;

    let body = resp.text().await.context("Read response failed")?;

    info!(
        response_body = %body,
        "Create dynamic response received"
    );

    let r: BilibiliCreateResponse =
        serde_json::from_str(&body).context("Parse create dynamic response failed")?;

    if r.code != 0 {
        return Err(AppError::InternalError(anyhow::anyhow!(
            "Bilibili API returned code {}",
            r.code
        )));
    }

    // Bilibili sometimes returns `code=0` but `data=null`.
    // Treat `code=0` as success and pass through the raw data.
    Ok(r.data
        .as_ref()
        .map(|d| serde_json::json!(d))
        .unwrap_or(serde_json::json!(null)))
}

/// Helper function to create dynamic with specified scene and optional pics
async fn create_dynamic_with_scene(
    contents: serde_json::Value,
    pics: Option<Vec<PicInfo>>,
    sessdata: &str,
    bili_jct: &str,
    client: &reqwest::Client,
) -> AppResult<Json<DynamicResponse>> {
    let upload_id = format!("{}_{}", get_unix_seconds(), get_nonce());

    let mut dyn_req_content = serde_json::json!({
        "dyn_req": {
            "content": {
                "contents": contents
            },
            "scene": if pics.is_some() {2} else {1},
            "attach_card": null,
            "upload_id": upload_id,
            "meta": {
                "app_meta": {
                    "from": "create.dynamic.web",
                    "mobi_app": "web"
                }
            }
        }
    });

    // Add pics field if provided
    if let Some(pics) = pics {
        dyn_req_content["dyn_req"]["pics"] =
            serde_json::to_value(pics).context("Failed to serialize pics")?;
    }

    let mut headers = create_headers(sessdata);
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let url = format!(
        "https://api.bilibili.com/x/dynamic/feed/create/dyn?platform=web&csrf={}",
        bili_jct
    );

    let result = client
        .post(&url)
        .headers(headers)
        .body(dyn_req_content.to_string())
        .send()
        .await;

    let data = handle_create_dynamic_response(result).await?;
    Ok(Json(DynamicResponse {
        code: 0,
        msg: None,
        data: Some(data),
        exception: None,
    }))
}

/// Create a Bilibili dynamic post with optional images
#[debug_handler]
#[utoipa::path(
    post,
    tag = "bilibili",
    path = "/bilibili/createDynamic",
    request_body(content_type = "multipart/form-data",
    description = "
- **msg** (required, string): JSON value that will be sent to Bilibili as `dyn_req.content.contents`. For example: `[{\"type\":1,\"raw_text\":\"Hello from Rust API!\",\"biz_id\":\"\"}]`.
- **file(s)** (optional): Any multipart field *with a filename* is treated as an uploaded image. The server does not require a specific field name like `files`, `image`, etc."
    ),

    responses(
        (status = OK, body = DynamicResponse),
        (status = UNAUTHORIZED, body = DynamicResponse),
        (status = BAD_REQUEST, body = DynamicResponse),
        (status = INTERNAL_SERVER_ERROR, body = DynamicResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_dynamic(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<DynamicResponse>> {
    // Extract Bilibili config
    let bilibili_config = &state.bilibili_config;

    let mut msg: Option<String> = None;
    let mut files: Vec<(Vec<u8>, String, String)> = Vec::new();

    // Parse multipart form data
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let field_name = field.name().unwrap_or("").to_string();

                match field_name.as_str() {
                    "msg" => {
                        msg = field.text().await.ok();
                    }
                    _ => {
                        // Assume it's a file upload
                        if let Some(file_name) = field.file_name() {
                            let file_name = file_name.to_string();
                            let content_type = field
                                .content_type()
                                .unwrap_or("application/octet-stream")
                                .to_string();
                            if let Ok(data) = field.bytes().await {
                                files.push((data.to_vec(), file_name, content_type));
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                // End of multipart fields
                break;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Error reading multipart field"
                );
                break;
            }
        }
    }

    // Validate msg
    let msg_content = msg
        .filter(|m| !m.is_empty())
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("need msg")))?;

    // Parse msg as JSON
    let contents: serde_json::Value =
        serde_json::from_str(&msg_content).context("Invalid msg format")?;

    // If files are present, upload them first
    if !files.is_empty() {
        info!(file_count = files.len(), "Uploading files");
        let mut pics: Vec<PicInfo> = Vec::new();

        for (file_data, file_name, content_type) in files {
            let (size, data) = upload_image(
                file_data,
                file_name,
                content_type,
                &bilibili_config.sessdata,
                &bilibili_config.bili_jct,
                &state.http_client,
            )
            .await?;

            pics.push(PicInfo {
                img_src: data.image_url,
                img_width: data.image_width,
                img_height: data.image_height,
                img_size: size,
            });
        }

        // Create dynamic with images (scene 2)
        create_dynamic_with_scene(
            contents,
            Some(pics),
            &bilibili_config.sessdata,
            &bilibili_config.bili_jct,
            &state.http_client,
        )
        .await
    } else {
        // Create text-only dynamic (scene 1)
        create_dynamic_with_scene(
            contents,
            None,
            &bilibili_config.sessdata,
            &bilibili_config.bili_jct,
            &state.http_client,
        )
        .await
    }
}
