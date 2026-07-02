mod routes;

use axum::Router;
use crate::api::routes::default_state;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;

pub fn router() -> Router {
    let state = default_state();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api", routes::api_routes())
        .nest_service("/static", {
        let cwd = std::env::current_dir().unwrap_or_default();
        let parent = cwd.parent().unwrap_or(&cwd);
        let static_path = parent.join("frontend").join("static");
        tracing::info!("Serving static files from: {:?}", static_path);
        ServeDir::new(static_path)
    })
        .merge(routes::page_routes())
        .with_state(state)
        .layer(cors)
}
