use std::net::SocketAddr;

use axum::{http::StatusCode, response::IntoResponse, Router};
use time::Duration;
use tokio::{signal, task::AbortHandle};
use tower_http::{
    compression::{CompressionLayer, CompressionLevel},
    trace::TraceLayer,
};
use tower_sessions::{session_store::ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing::{debug, instrument};

use crate::api;
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

async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
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
        () = ctrl_c => { deletion_task_abort_handle.abort() },
        () = terminate => { deletion_task_abort_handle.abort() },
    }

    println!("signal received, starting graceful shutdown");
}

#[instrument(skip_all)]
pub async fn start_server(db: Database) -> miette::Result<(), Error> {
    debug!("starting http server");

    let app_state = State {
        database: db.clone(),
    };

    let auth_router = api::auth::router();
    let api_v1_router = api::v1::router();

    // Set up the session layer
    let session_store = PostgresStore::new(db.clone().0);
    session_store.migrate().await?;

    let deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
    );

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(10)));

    let app = Router::new()
        .fallback(not_found)
        .with_state(app_state)
        .nest("/api/auth", auth_router)
        .nest("/api/v1", api_v1_router)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new().quality(CompressionLevel::Fastest));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    debug!("binding to {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    debug!("listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        .await?;

    deletion_task.await.unwrap()?;

    Ok(())
}
