use std::net::SocketAddr;

use axum::{extract::FromRef, http::StatusCode, response::IntoResponse, Router};
use axum_login::AuthManagerLayerBuilder;
use miette::IntoDiagnostic as _;
use time::Duration;
use tokio::{signal, task::AbortHandle};
use tower_http::{
    compression::{CompressionLayer, CompressionLevel},
    trace::TraceLayer,
};
use tower_sessions::{session_store::ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing::{debug, instrument};

use crate::Database;
use crate::{api, auth::Authenticator};

#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub authenticator: Authenticator,
    pub session_store: PostgresStore,
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

    println!("signal received, gracefully shutting down");
}

#[instrument(skip_all)]
pub async fn start_server(db: Database, authenticator: Authenticator) -> miette::Result<()> {
    debug!("starting http server");

    let auth_router = api::auth::router();
    let api_v1_router = api::v1::router();

    // Set up the session layer
    debug!("creating session store");
    let session_store = PostgresStore::new(db.clone());
    debug!("migrating session store");
    session_store.migrate().await.into_diagnostic()?;

    let app_state = AppState {
        authenticator: authenticator.clone(),
        session_store: session_store.clone(),
        database: db.clone(),
    };

    let deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(360)),
    );

    let session_layer = SessionManagerLayer::new(session_store.clone())
        .with_secure(false)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(360)));

    let auth_layer = AuthManagerLayerBuilder::new(authenticator, session_layer).build();

    let app = Router::new()
        .fallback(not_found)
        .nest("/api/v1", api_v1_router)
        .nest("/api/auth", auth_router)
        .with_state(app_state)
        .layer(auth_layer)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new().quality(CompressionLevel::Fastest));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    debug!("binding to {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .into_diagnostic()?;
    debug!("listening on {}", addr);
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        .await
        .into_diagnostic()?;

    deletion_task.await.into_diagnostic()?.into_diagnostic()?;

    Ok(())
}
