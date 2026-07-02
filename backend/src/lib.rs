pub mod models;
pub mod session;
pub mod api;

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn run() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "omp_visualizer=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let app = api::router();

        let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
        tracing::info!("omp-visualizer listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
}
