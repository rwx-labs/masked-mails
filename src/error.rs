//! Error types

use std::io;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error("Cannot connect to database database")]
    #[diagnostic(code(masked_mails::db_open))]
    DatabaseOpenError(#[source] sqlx::Error),
    #[error("Could not acquire a connection from the connection pool")]
    DatabaseConnAcqError(#[source] sqlx::Error),
    #[error("Database migration failed")]
    DatabaseMigrationError(#[source] sqlx::migrate::MigrateError),
    #[error("Database query failed")]
    DatabaseQueryFailed(#[from] sqlx::Error),
    #[error("Could not bind port for http server")]
    HttpBindFailed(#[source] io::Error),
    #[error("Could not discover openid client information")]
    DiscoverOidcFailed,
}
