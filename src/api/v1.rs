use axum::{routing::get, Router};

mod address;

pub fn router() -> Router<crate::http::AppState> {
    Router::new()
        .route(
            "/addresses",
            get(handlers::list_addresses).post(handlers::create_address),
        )
        .route(
            "/addresses/:id",
            get(handlers::get_address).delete(handlers::delete_address),
        )
}

mod handlers {
    use axum::{
        extract::{Json, Path, State},
        http::StatusCode,
        response::IntoResponse,
    };
    use serde::Deserialize;
    use tracing::{error, instrument};

    use crate::{auth::AuthSession, http::AppState};

    use super::address;

    #[derive(Clone, Deserialize, Debug)]
    pub struct CreateAddressRequest {
        pub domain_id: i32,
        pub description: Option<String>,
    }

    #[instrument]
    pub(super) async fn list_addresses(
        auth_session: AuthSession,
        State(AppState { database, .. }): State<AppState>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                if let Ok(addrs) = address::get_user_addresses(user.id, &database).await {
                    Json(addrs).into_response()
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                }
            }
            None => (StatusCode::UNAUTHORIZED).into_response(),
        }
    }

    #[instrument]
    pub(super) async fn get_address(
        Path(address_id): Path<i32>,
        auth_session: AuthSession,
        State(AppState { database, .. }): State<AppState>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => match address::get_user_address(user.id, address_id, &database).await {
                Ok(Some(addr)) => (StatusCode::OK, Json(addr)).into_response(),
                Ok(None) => (StatusCode::NOT_FOUND).into_response(),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            },
            None => (StatusCode::UNAUTHORIZED).into_response(),
        }
    }

    #[instrument]
    #[axum::debug_handler]
    pub(super) async fn create_address(
        auth_session: AuthSession,
        State(AppState { database, .. }): State<AppState>,
        Json(request): Json<CreateAddressRequest>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let random_addr =
                    match address::generate_domain_address(request.domain_id, &database).await {
                        Ok(addr) => addr,
                        Err(err) => {
                            error!(?err, "could not generate random address for domain");
                            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                        }
                    };

                let addr = address::CreateAddress {
                    address: random_addr,
                    description: request.description,
                    enabled: true,
                    domain_id: request.domain_id,
                    user_id: user.id,
                };

                match address::create_address(addr, &database).await {
                    Ok(addr) => (StatusCode::OK, Json(addr)).into_response(),
                    Err(err) => {
                        error!(?err, "could not create address");

                        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                    }
                }
            }
            None => (StatusCode::UNAUTHORIZED).into_response(),
        }
    }

    #[instrument]
    pub(super) async fn delete_address(
        Path(address_id): Path<i32>,
        auth_session: AuthSession,
        State(AppState { database, .. }): State<AppState>,
    ) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                match address::delete_user_address(user.id, address_id, &database).await {
                    Ok(Some(addr)) => (StatusCode::OK, Json(addr)).into_response(),
                    Ok(None) => (StatusCode::NOT_FOUND).into_response(),
                    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
                }
            }
            None => (StatusCode::UNAUTHORIZED).into_response(),
        }
    }
}
