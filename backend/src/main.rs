mod api;
mod models;
mod session;

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "omp_visualizer=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = api::router();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("omp-visualizer listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
