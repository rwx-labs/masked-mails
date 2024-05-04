use std::ops::Deref;
use std::time::Duration;

use sqlx::{
    migrate::Migrator,
    postgres::{PgPool, PgPoolOptions},
};

use crate::Error;

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Debug, Clone)]
pub struct Database(pub PgPool);

impl Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn connect(url: &str) -> Result<Database, Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(Duration::from_secs(30))
        .connect(url)
        .await
        .map_err(Error::DatabaseOpenError)
        .map(Database)?;

    Ok(pool)
}

pub async fn migrate(pool: Database) -> Result<(), Error> {
    let mut conn = pool.acquire().await.map_err(Error::DatabaseConnAcqError)?;

    MIGRATOR
        .run(&mut conn)
        .await
        .map_err(Error::DatabaseMigrationError)
}
