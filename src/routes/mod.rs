#![allow(clippy::needless_for_each)]
mod aliyun_handlers;
mod bilibili_handlers;
mod misc_handlers;
use crate::{auth::jwt_auth_middleware, middleware::apply_axum_middleware, state::AppState};
use axum::{Json, Router, middleware, routing::get};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "bilibili", description = "Bilibili dynamic posting endpoints"),
        (name = "aliyun", description = "Aliyun CDN API endpoints"),
    ),
    components(
        schemas(
            bilibili_handlers::DynamicResponse,
            aliyun_handlers::DescribeRefreshTasksPayload,
            aliyun_handlers::RefreshObjectCachesPayload,
            aliyun_handlers::OssEventPayload,
            aliyun_handlers::OssEventResponse,
            aliyun_handlers::OssEventData,
            aliyun_handlers::OssData,
            aliyun_handlers::OssBucket,
            aliyun_handlers::OssObject,
            crate::aliyun::DescribeRefreshTasksResponse,
            crate::aliyun::RefreshObjectCachesResponse,
            crate::aliyun::cdn::TasksContainer,
            crate::aliyun::cdn::RefreshTask,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
            components.add_security_scheme(
                "eventbridge_token",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Header(
                        utoipa::openapi::security::ApiKeyValue::new(
                            "x-eventbridge-signature-token",
                        ),
                    ),
                ),
            );
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    // Routes without JWT auth (public + custom auth)
    let (public_routes, openapi_public) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        // Health endpoints (no auth required)
        .routes(routes!(misc_handlers::ping))
        .routes(routes!(misc_handlers::health))
        // Aliyun EventBridge endpoint with custom JWT auth via `x-eventbridge-signature-token` header
        .routes(routes!(aliyun_handlers::handle_oss_events))
        .split_for_parts();

    // Routes protected by Authorization header JWT
    let (protected_routes, openapi_protected) = OpenApiRouter::new()
        // Bilibili routes (protected by JWT auth)
        .routes(routes!(bilibili_handlers::create_dynamic))
        // Aliyun routes (protected by JWT auth)
        .routes(routes!(aliyun_handlers::describe_refresh_tasks))
        .routes(routes!(aliyun_handlers::refresh_object_caches))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ))
        .split_for_parts();

    // Merge OpenAPI specs
    let mut openapi = openapi_public;
    openapi.merge(openapi_protected);

    // Merge route handlers
    let api_routes = public_routes.merge(protected_routes);

    openapi.paths.paths = openapi
        .paths
        .paths
        .into_iter()
        .map(|(path, item)| (format!("/api{path}"), item))
        .collect::<utoipa::openapi::path::PathsMap<_, _>>();
    let full_router = Router::new()
        .nest("/api", api_routes)
        .merge(Scalar::with_url("/api/scalar", openapi.clone()))
        .route("/api/openapi.json", get(|| async move { Json(openapi) }))
        .with_state(state);

    // Apply middleware
    apply_axum_middleware(full_router)
}
