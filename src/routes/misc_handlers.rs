use axum::{Json, debug_handler};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize)]
pub struct Health {
    pub ok: bool,
}

/// /_ping
#[debug_handler]
#[utoipa::path(get, path = "/_ping", tag = "health", responses((status = OK, body = Health)))]
pub async fn ping() -> Json<Health> {
    Json(Health { ok: true })
}
