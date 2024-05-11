use ::tracing::debug;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use miette::IntoDiagnostic;

mod api;
mod auth;
mod cli;
mod config;
mod database;
mod error;
mod http;
mod tracing;

pub use config::Config;
pub use database::Database;
pub use error::Error;

use crate::auth::Authenticator;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let opts: cli::Opts = argh::from_env();
    let config: Config = Figment::new()
        .merge(Toml::file(opts.config_path))
        .merge(Env::prefixed("MM_").lowercase(false).split("__"))
        .extract()
        .into_diagnostic()?;

    tracing::init(&config.tracing)?;

    debug!("connecting to database");
    let db = database::connect(config.database.url.as_str(), &config.database).await?;
    debug!("connected to database");

    debug!("running database migrations");
    database::migrate(db.clone()).await?;
    debug!("database migrations complete");

    debug!("configuring authenticator");
    let authenticator = Authenticator::discover(
        db.clone(),
        config.auth.issuer_url.clone(),
        config.auth.client_id.clone(),
        config.auth.client_secret.clone(),
        config.auth.redirect_url.clone(),
    )
    .await?;
    debug!("finished configuration authenticator");

    http::start_server(db.clone(), authenticator, config).await?;

    Ok(())
}
