use axum::{
    Json, debug_handler,
    extract::{Multipart, State},
    http::StatusCode,
};
use rand::Rng;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use utoipa::ToSchema;

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
) -> Result<(f64, BilibiliUploadData), String> {
    let file_size_kb = file_data.len() as f64 / 1024.0;

    let file_part = Part::bytes(file_data)
        .file_name(file_name)
        .mime_str(&content_type)
        .map_err(|e| format!("Failed to create file part: {}", e))?;

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
        .map_err(|e| format!("Upload request failed: {}", e))?;

    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let upload_resp: BilibiliUploadResponse = serde_json::from_str(&resp_text)
        .map_err(|e| format!("Failed to parse upload response: {}", e))?;

    if upload_resp.code != 0 {
        return Err(format!(
            "Bilibili file upload failed, response: {}",
            resp_text
        ));
    }

    let data = upload_resp
        .data
        .ok_or_else(|| "Upload response missing data".to_string())?;

    Ok((file_size_kb, data))
}

/// POST /createDynamic - Create a Bilibili dynamic post with optional images
#[debug_handler]
#[utoipa::path(
    post,
    path = "/createDynamic",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = OK, body = DynamicResponse),
        (status = BAD_REQUEST, body = DynamicResponse),
        (status = INTERNAL_SERVER_ERROR, body = DynamicResponse)
    )
)]
pub async fn create_dynamic(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> (StatusCode, Json<DynamicResponse>) {
    // Extract Bilibili config
    let bilibili_config = &state.bilibili_config;

    let mut msg: Option<String> = None;
    let mut files: Vec<(Vec<u8>, String, String)> = Vec::new();

    // Parse multipart form data
    while let Ok(Some(field)) = multipart.next_field().await {
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

    // Validate msg
    let msg_content = match msg {
        Some(m) if !m.is_empty() => m,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(DynamicResponse {
                    code: 1,
                    msg: Some("need msg".to_string()),
                    data: None,
                    exception: None,
                }),
            );
        }
    };

    // Parse msg as JSON
    let contents: serde_json::Value = match serde_json::from_str(&msg_content) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to parse msg as JSON: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(DynamicResponse {
                    code: 1,
                    msg: Some(format!("Invalid msg format: {}", e)),
                    data: None,
                    exception: None,
                }),
            );
        }
    };

    // If files are present, upload them first
    if !files.is_empty() {
        info!("Uploading {} files", files.len());
        let mut pics: Vec<PicInfo> = Vec::new();

        for (file_data, file_name, content_type) in files {
            match upload_image(
                file_data,
                file_name,
                content_type,
                &bilibili_config.sessdata,
                &bilibili_config.bili_jct,
                &state.http_client,
            )
            .await
            {
                Ok((size, data)) => {
                    pics.push(PicInfo {
                        img_src: data.image_url,
                        img_width: data.image_width,
                        img_height: data.image_height,
                        img_size: size,
                    });
                }
                Err(e) => {
                    error!("Upload file failed: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(DynamicResponse {
                            code: 1,
                            msg: Some("upload file fail".to_string()),
                            data: None,
                            exception: Some(serde_json::json!({ "error": e })),
                        }),
                    );
                }
            }
        }

        // Create dynamic with images (scene 2)
        let upload_id = format!("{}_{}", get_unix_seconds(), get_nonce());

        let dyn_req = serde_json::json!({
            "dyn_req": {
                "content": {
                    "contents": contents
                },
                "scene": 2,
                "attach_card": null,
                "upload_id": upload_id,
                "meta": {
                    "app_meta": {
                        "from": "create.dynamic.web",
                        "mobi_app": "web"
                    }
                },
                "pics": pics
            }
        });

        let mut headers = create_headers(&bilibili_config.sessdata);
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let url = format!(
            "https://api.bilibili.com/x/dynamic/feed/create/dyn?platform=web&csrf={}",
            bilibili_config.bili_jct
        );

        match state
            .http_client
            .post(&url)
            .headers(headers)
            .body(dyn_req.to_string())
            .send()
            .await
        {
            Ok(resp) => match resp.text().await {
                Ok(body) => {
                    info!("Create dynamic response: {}", body);
                    match serde_json::from_str::<BilibiliCreateResponse>(&body) {
                        Ok(r) => {
                            if r.code != 0 {
                                return (
                                    StatusCode::OK,
                                    Json(DynamicResponse {
                                        code: 1,
                                        msg: None,
                                        data: None,
                                        exception: Some(serde_json::json!(r)),
                                    }),
                                );
                            }

                            // Bilibili sometimes returns `code=0` but `data=null`.
                            // Treat `code=0` as success and pass through the raw data.
                            (
                                StatusCode::OK,
                                Json(DynamicResponse {
                                    code: 0,
                                    msg: None,
                                    data: r.data.as_ref().map(|d| serde_json::json!(d)),
                                    exception: None,
                                }),
                            )
                        }
                        Err(e) => {
                            error!("Parse create dynamic response failed: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(DynamicResponse {
                                    code: 1,
                                    msg: Some("create dynamic fail".to_string()),
                                    data: None,
                                    exception: Some(
                                        serde_json::json!({ "body": body, "error": e.to_string() }),
                                    ),
                                }),
                            )
                        }
                    }
                }
                Err(e) => {
                    error!("Read response failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(DynamicResponse {
                            code: 1,
                            msg: Some("create dynamic fail with network fatal".to_string()),
                            data: None,
                            exception: Some(serde_json::json!({ "error": e.to_string() })),
                        }),
                    )
                }
            },
            Err(e) => {
                error!("Create dynamic request failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DynamicResponse {
                        code: 1,
                        msg: Some("create dynamic fail with network fatal".to_string()),
                        data: None,
                        exception: Some(serde_json::json!({ "error": e.to_string() })),
                    }),
                )
            }
        }
    } else {
        // Create text-only dynamic (scene 1)
        let upload_id = format!("{}_{}", get_unix_seconds(), get_nonce());

        let dyn_req = serde_json::json!({
            "dyn_req": {
                "content": {
                    "contents": contents
                },
                "scene": 1,
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

        let mut headers = create_headers(&bilibili_config.sessdata);
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let url = format!(
            "https://api.bilibili.com/x/dynamic/feed/create/dyn?platform=web&csrf={}",
            bilibili_config.bili_jct
        );

        match state
            .http_client
            .post(&url)
            .headers(headers)
            .body(dyn_req.to_string())
            .send()
            .await
        {
            Ok(resp) => match resp.text().await {
                Ok(body) => {
                    info!("Create dynamic response: {}", body);
                    match serde_json::from_str::<BilibiliCreateResponse>(&body) {
                        Ok(r) => {
                            if r.code != 0 {
                                return (
                                    StatusCode::OK,
                                    Json(DynamicResponse {
                                        code: 1,
                                        msg: None,
                                        data: None,
                                        exception: Some(serde_json::json!(r)),
                                    }),
                                );
                            }

                            // Bilibili sometimes returns `code=0` but incomplete/partial data.
                            // Treat `code=0` as success and pass through the raw data.
                            (
                                StatusCode::OK,
                                Json(DynamicResponse {
                                    code: 0,
                                    msg: None,
                                    data: r.data.as_ref().map(|d| serde_json::json!(d)),
                                    exception: None,
                                }),
                            )
                        }
                        Err(e) => {
                            error!("Parse create dynamic response failed: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(DynamicResponse {
                                    code: 1,
                                    msg: Some("create dynamic fail".to_string()),
                                    data: None,
                                    exception: Some(serde_json::json!({ "error": e.to_string() })),
                                }),
                            )
                        }
                    }
                }
                Err(e) => {
                    error!("Read response failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(DynamicResponse {
                            code: 1,
                            msg: Some("create dynamic fail".to_string()),
                            data: None,
                            exception: Some(serde_json::json!({ "error": e.to_string() })),
                        }),
                    )
                }
            },
            Err(e) => {
                error!("Create dynamic request failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DynamicResponse {
                        code: 1,
                        msg: Some("create dynamic fail".to_string()),
                        data: None,
                        exception: Some(serde_json::json!({ "error": e.to_string() })),
                    }),
                )
            }
        }
    }
}
