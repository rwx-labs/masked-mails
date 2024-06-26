use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    routing::{get, post},
    Router,
};
use axum_login::login_required;

use crate::auth::Authenticator;

mod address;
mod domain;

pub fn router() -> Router<crate::http::AppState> {
    Router::new()
        // These routes require login
        .route(
            "/addresses",
            get(handlers::list_addresses).post(handlers::create_address),
        )
        .route(
            "/addresses/:id",
            get(handlers::get_address).delete(handlers::delete_address),
        )
        // The routes following this layer do not require login
        .route_layer(login_required!(
            Authenticator,
            login_url = "/api/auth/login"
        ))
        .route("/domains", get(handlers::list_domains))
        .route("/domains/:id", get(handlers::get_domain))
        // The ingress route implements its own auth check
        .route("/ingestion", post(handlers::ingest))
}

struct ExtractAuthToken(String);

#[async_trait]
impl<S> FromRequestParts<S> for ExtractAuthToken
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(auth) = parts.headers.get(AUTHORIZATION) {
            let value = auth.as_bytes();

            if value.starts_with(b"Token ") {
                let token = String::from_utf8_lossy(&value[6..]);

                Ok(ExtractAuthToken(token.into_owned()))
            } else {
                Err((StatusCode::BAD_REQUEST, "invalid authorization scheme"))
            }
        } else {
            Err((StatusCode::BAD_REQUEST, "authorization token is missing"))
        }
    }
}

mod handlers {
    use std::collections::HashMap;

    use axum::{
        extract::{Json, Path, State},
        http::StatusCode,
        response::IntoResponse,
    };
    use base64::prelude::{Engine, BASE64_STANDARD};
    use mail_parser::MessageParser;
    use serde::Deserialize;
    use tracing::{debug, error, instrument};

    use crate::{auth::AuthSession, http::AppState};

    use super::{address, domain, ExtractAuthToken};

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

    #[instrument]
    pub(super) async fn list_domains(
        State(AppState { database, .. }): State<AppState>,
    ) -> impl IntoResponse {
        match domain::get_domains(&database).await {
            Ok(domains) => (StatusCode::OK, Json(domains)).into_response(),
            Err(err) => {
                error!(?err, "could not fetch list of domains");

                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }

    #[instrument]
    pub(super) async fn get_domain(
        Path(domain_id): Path<i32>,
        State(AppState { database, .. }): State<AppState>,
    ) -> impl IntoResponse {
        match domain::get_domain(domain_id, &database).await {
            Ok(Some(domain)) => (StatusCode::OK, Json(domain)).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND).into_response(),
            Err(err) => {
                error!(?err, %domain_id, "could not fetch domain");

                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct MailMetadata {
        /// The intended recipient, if known.
        pub to: Option<String>,
        /// The sender, if known.
        pub from: Option<String>,
        /// E-mail headers, if known.
        pub headers: HashMap<String, String>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Mail {
        /// The raw contents of the e-mail, encoded with base64.
        pub raw: String,
        /// The size of the (decoded) raw contents.
        pub raw_size: usize,
        /// Information about the e-mail that was known prior to parsing.
        pub metadata: MailMetadata,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct MailIngestionRequest {
        pub mails: Vec<Mail>,
        pub started_at: String, // FIXME: this should be deserialized to a time
    }

    #[instrument(skip_all)]
    pub(super) async fn ingest(
        State(AppState { config, .. }): State<AppState>,
        ExtractAuthToken(token): ExtractAuthToken,
        Json(payload): Json<MailIngestionRequest>,
    ) -> impl IntoResponse {
        if token != config.ingestion.api_token {
            debug!("received ingestion request with invalid token");

            return (StatusCode::UNAUTHORIZED, "invalid authorization token").into_response();
        }

        let mail_parser = MessageParser::new()
            .with_mime_headers()
            .with_date_headers()
            .with_address_headers()
            .with_message_ids();

        for mail in &payload.mails {
            let decoded = match BASE64_STANDARD.decode(&mail.raw) {
                Ok(data) => data,
                Err(_) => continue,
            };

            match mail_parser.parse(&decoded[..]) {
                Some(parsed) => {
                    debug!(?parsed, "parsed email");
                }
                None => {
                    error!("could not parse email");
                }
            }
        }

        ().into_response()

        // debug!(%raw, %raw_size, %to, %from, "received email");

        // match MessageParser::default().parse(&raw) {
        //     Some(_parsed) => {
        //         debug!("parsed email");

        //         ().into_response()
        //     }
        //     None => {
        //         error!("could not parse email");

        //         (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        //     }
        // }
    }
}
