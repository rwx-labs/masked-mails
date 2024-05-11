use std::time::Duration;

use sqlx::{
    migrate::Migrator,
    postgres::{PgPool, PgPoolOptions},
};

use crate::Error;

static MIGRATOR: Migrator = sqlx::migrate!();

pub type Database = PgPool;

pub async fn connect(url: &str) -> Result<Database, Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(Duration::from_secs(30))
        .connect(url)
        .await
        .map_err(Error::DatabaseOpenError)?;

    Ok(pool)
}

pub async fn migrate(pool: Database) -> Result<(), Error> {
    let mut conn = pool.acquire().await.map_err(Error::DatabaseConnAcqError)?;

    MIGRATOR
        .run(&mut conn)
        .await
        .map_err(Error::DatabaseMigrationError)
}
