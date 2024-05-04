use axum::{routing::get, Router};

pub fn router() -> Router<crate::http::AppState> {
    Router::new()
        .route("/callback", get(handlers::callback))
        .route("/login", get(handlers::login))
}

mod handlers {
    use axum::{
        extract::State,
        http::StatusCode,
        response::{IntoResponse, Redirect},
    };
    use tracing::instrument;

    use crate::auth::Authenticator;

    #[instrument]
    pub(super) async fn callback() -> (StatusCode, String) {
        (
            StatusCode::NOT_IMPLEMENTED,
            "not yet implemented".to_string(),
        )
    }

    #[instrument(skip(auth))]
    pub(super) async fn login(State(auth): State<Authenticator>) -> impl IntoResponse {
        let auth_url = auth.login_redirect_url().await;

        Redirect::to(auth_url.as_str())
    }
}
