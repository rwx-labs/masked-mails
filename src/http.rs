use std::net::SocketAddr;

use axum::{http::StatusCode, response::IntoResponse, Router};
use tokio::signal;
use tower_http::{
    compression::{CompressionLayer, CompressionLevel},
    trace::TraceLayer,
};
use tracing::{debug, instrument};

use crate::Database;
use crate::Error;

#[derive(Clone)]
struct State {
    pub database: Database,
}

#[instrument]
async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "404 page not found")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}

#[instrument(skip_all)]
pub async fn start_server(db: Database) -> Result<(), Error> {
    debug!("starting http server");

    let app_state = State { database: db };

    let api_v1_router = crate::api::v1::router();

    let app = Router::new()
        .fallback(not_found)
        .with_state(app_state)
        .nest("/api/v1", api_v1_router)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new().quality(CompressionLevel::Fastest));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    debug!("binding to {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    debug!("listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}
