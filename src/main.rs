use std::net::SocketAddr;

use axum::{http::StatusCode, response::IntoResponse, Router};
use clap::Parser;
use opentelemetry::KeyValue;
use opentelemetry_sdk::{
    trace::{BatchConfig, RandomIdGenerator},
    Resource,
};
use opentelemetry_semantic_conventions::{
    resource::{SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};

use tokio::signal;
use tower_http::{
    compression::{CompressionLayer, CompressionLevel},
    trace::TraceLayer,
};
use tracing::{debug, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod database;
mod error;

pub use error::Error;

#[derive(Clone)]
struct AppState {
    pub database: database::Database,
}

#[instrument]
async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
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

// Create a Resource that captures information about the entity for which telemetry is recorded.
fn resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ],
        SCHEMA_URL,
    )
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let opts = cli::Opts::parse();

    // Create a tracing layer with the configured tracer
    let telemetry_layer = if opts.tracing {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(resource()),
            )
            .with_batch_config(BatchConfig::default())
            .with_exporter(opentelemetry_otlp::new_exporter().tonic())
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("could not create otlp pipeline");
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    };

    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "magistr=debug,tower_http=debug".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(false)
                .with_span_list(false),
        )
        .with(telemetry_layer)
        .init();

    debug!("connecting to database");
    let db = database::connect(opts.database_url.as_str()).await?;
    debug!("connected to database");

    debug!("running database migrations");
    database::migrate(db.clone()).await?;
    debug!("database migrations complete");
    let app_state = AppState { database: db };
    let app = Router::new()
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new().quality(CompressionLevel::Fastest))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    debug!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}
