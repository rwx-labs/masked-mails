use rand::{
    distributions::{Alphanumeric, DistString},
    Rng,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::Error;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Address {
    pub id: i32,
    pub address: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub domain_id: i32,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAddress {
    pub address: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub domain_id: i32,
    pub user_id: i32,
}

/// Returns the address with `address_id` that belongs to `user_id`.
pub async fn get_user_address(
    user_id: i32,
    address_id: i32,
    db: &crate::Database,
) -> Result<Option<Address>, Error> {
    let addr = sqlx::query_as("SELECT * FROM addresses WHERE user_id = $1 AND id = $2")
        .bind(user_id)
        .bind(address_id)
        .fetch_optional(db)
        .await?;

    Ok(addr)
}

/// Returns a list of all addresses belonging to `user_id`.
pub async fn get_user_addresses(user_id: i32, db: &crate::Database) -> Result<Vec<Address>, Error> {
    let addrs: Vec<Address> = sqlx::query_as("SELECT * FROM addresses WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(db)
        .await?;

    Ok(addrs)
}

/// Returns a list of all addresses belonging to `user_id`.
pub async fn create_address(addr: CreateAddress, db: &crate::Database) -> Result<Address, Error> {
    let result = sqlx::query_as(
        r"
        INSERT INTO addresses (address, description, enabled, domain_id, user_id) VALUES (
            $1, $2, $3, $4, $5
        ) RETURNING *
        ",
    )
    .bind(addr.address)
    .bind(addr.description)
    .bind(addr.enabled)
    .bind(addr.domain_id)
    .bind(addr.user_id)
    .fetch_one(db)
    .await?;

    Ok(result)
}

/// Deletes the address with the given `address_id` belonging to `user_id` and returns the address
/// that was deleted, if any.
pub async fn delete_user_address(
    user_id: i32,
    address_id: i32,
    db: &crate::Database,
) -> Result<Option<Address>, Error> {
    let addr = sqlx::query_as("DELETE FROM addresses WHERE user_id = $1 AND id = $2 RETURNING *")
        .bind(user_id)
        .bind(address_id)
        .fetch_optional(db)
        .await?;

    Ok(addr)
}

/// Generates a unique address for the given domain.
// FIXME: avoid TOFU
pub async fn generate_domain_address(
    domain_id: i32,
    db: &crate::Database,
) -> Result<String, Error> {
    const MAX_ROUNDS: usize = 10;

    for _ in 0..MAX_ROUNDS {
        let addr = {
            let mut rng = rand::thread_rng();
            let length = rng.gen_range(10..=18);
            Alphanumeric.sample_string(&mut rng, length)
        };

        // Check that the address doesn't exist
        match sqlx::query_as::<_, (i32,)>(
            "SELECT id FROM addresses WHERE domain_id = $1 AND address = $2",
        )
        .bind(domain_id)
        .bind(&addr)
        .fetch_optional(db)
        .await?
        {
            Some(_) => {}
            None => {
                return Ok(addr);
            }
        }
    }

    Err(Error::NameCollisionLimit)
}
