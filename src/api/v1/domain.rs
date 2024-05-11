use serde::Serialize;
use sqlx::FromRow;

use crate::Error;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Domain {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
}

/// Returns the domain with the given `domain_id`.
pub async fn get_domain(domain_id: i32, db: &crate::Database) -> Result<Option<Domain>, Error> {
    let addr = sqlx::query_as("SELECT * FROM domains WHERE id = $1")
        .bind(domain_id)
        .fetch_optional(db)
        .await?;

    Ok(addr)
}

/// Returns a list of all domains.
pub async fn get_domains(db: &crate::Database) -> Result<Vec<Domain>, Error> {
    let addr = sqlx::query_as("SELECT * FROM domains")
        .fetch_all(db)
        .await?;

    Ok(addr)
}
