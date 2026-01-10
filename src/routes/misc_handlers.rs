use crate::{error::AppResult, repository::Repository, state::AppState};
use axum::{Json, debug_handler, extract::State};
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

/// /_health
#[debug_handler]
#[utoipa::path(get, path = "/_health", tag = "health", responses((status = OK, body = Health)))]
pub async fn health(State(state): State<AppState>) -> AppResult<Json<Health>> {
    Ok(Json(Health {
        ok: state.repository.health_check().await,
    }))
}
