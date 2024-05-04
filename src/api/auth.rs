use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/callback", get(handlers::callback))
        .route("/login", get(handlers::login))
}

mod handlers {
    use axum::http::StatusCode;
    use tracing::instrument;

    #[instrument]
    pub(super) async fn callback() -> (StatusCode, String) {
        (
            StatusCode::NOT_IMPLEMENTED,
            "not yet implemented".to_string(),
        )
    }

    #[instrument]
    pub(super) async fn login() -> (StatusCode, String) {
        (
            StatusCode::NOT_IMPLEMENTED,
            "not yet implemented".to_string(),
        )
    }
}
