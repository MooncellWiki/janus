#![allow(clippy::needless_for_each)]
mod bilibili_handlers;
mod misc_handlers;
use crate::{middleware::apply_axum_middleware, state::AppState};
use axum::{Json, Router, routing::get};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "bilibili", description = "Bilibili dynamic posting endpoints"),
    ),
)]
pub struct ApiDoc;

pub fn build_router(state: AppState) -> Router {
    let (api_routes, mut openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        // Health endpoints
        .routes(routes!(misc_handlers::ping))
        .routes(routes!(misc_handlers::health))
        // Bilibili endpoints
        .routes(routes!(bilibili_handlers::create_dynamic))
        .split_for_parts();

    openapi.paths.paths = openapi
        .paths
        .paths
        .into_iter()
        .map(|(path, item)| (format!("/api/v1{path}"), item))
        .collect::<utoipa::openapi::path::PathsMap<_, _>>();
    let full_router = Router::new()
        .nest("/api/v1", api_routes)
        .merge(Scalar::with_url("/api/v1/scalar", openapi.clone()))
        .route("/api/v1/openapi.json", get(|| async move { Json(openapi) }))
        .with_state(state);

    // Apply middleware
    apply_axum_middleware(full_router)
}
