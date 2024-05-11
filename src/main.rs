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

use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod auth;
mod cli;
mod database;
mod error;
mod http;

pub use database::Database;
pub use error::Error;

use crate::auth::Authenticator;

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
                .unwrap_or_else(|_| "masked_mails=debug,tower_http=debug".into()),
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

    debug!("configuring authenticator");
    let authenticator = Authenticator::discover(
        db.clone(),
        opts.auth_issuer_url,
        opts.auth_client_id,
        opts.auth_client_secret,
        opts.auth_redirect_url,
    )
    .await?;
    debug!("finished configuration authenticator");

    http::start_server(db.clone(), authenticator, opts.ingress_api_token).await?;

    Ok(())
}
