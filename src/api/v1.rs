use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route(
            "/addresses",
            get(handlers::list_addresses).post(handlers::create_address),
        )
        .route("/addresses/:id", get(handlers::get_address))
}

mod handlers {
    use axum::{
        extract::{Json, Path},
        http::StatusCode,
    };
    use serde::Deserialize;
    use tracing::instrument;

    #[derive(Deserialize, Debug)]
    pub struct CreateAddress {
        pub description: Option<String>,
    }

    #[instrument]
    pub(super) async fn list_addresses() -> (StatusCode, String) {
        (
            StatusCode::NOT_IMPLEMENTED,
            "not yet implemented".to_string(),
        )
    }

    #[instrument]
    pub(super) async fn get_address(Path(id): Path<u64>) -> (StatusCode, String) {
        (
            StatusCode::NOT_IMPLEMENTED,
            "not yet implemented".to_string(),
        )
    }

    #[instrument]
    pub(super) async fn create_address(Json(payload): Json<CreateAddress>) -> (StatusCode, String) {
        (
            StatusCode::NOT_IMPLEMENTED,
            "not yet implemented".to_string(),
        )
    }
}
