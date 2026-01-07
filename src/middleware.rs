use axum::Router;
use std::time::Duration;
use tower_http::{compression::CompressionLayer, timeout::RequestBodyTimeoutLayer};

pub fn apply_axum_middleware(router: Router) -> Router {
    router
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(10)))
        .layer(CompressionLayer::new())
}
