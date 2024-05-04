//! OIDC authentication

use std::{collections::HashMap, sync::Arc};

use openidconnect::reqwest::async_http_client;
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    url::Url,
    ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl, Scope,
};
use tokio::sync::RwLock;
use tracing::debug;

use crate::Error;

type OidcStore = Arc<RwLock<HashMap<String, Nonce>>>;

#[derive(Clone, Debug)]
pub struct Authenticator {
    pub client: CoreClient,
    store: OidcStore,
}

impl Authenticator {
    /// Create an Authenticator based on the OpenID Connect Discovery document.
    pub(crate) async fn discover(
        url: Url,
        client_id: String,
        client_secret: String,
        redirect_url: Url,
    ) -> Result<Self, Error> {
        debug!("running openid connect discovery");
        let issuer_url = IssuerUrl::from_url(url);
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
            .await
            .map_err(|_| Error::DiscoverOidcFailed)?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
        )
        .set_redirect_uri(RedirectUrl::from_url(redirect_url));

        let store = Arc::new(RwLock::new(HashMap::new()));

        debug!("finished openid connect discovery");

        Ok(Authenticator { client, store })
    }

    pub async fn login_redirect_url(&self) -> Url {
        // Generate the full authorization URL.
        let (auth_url, csrf_token, nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            // Set the desired scopes.
            .add_scope(Scope::new("openid".to_string()))
            .url();

        self.store
            .write()
            .await
            .insert(csrf_token.secret().to_owned(), nonce);

        tracing::debug!("{:?}", &self.store);

        auth_url
    }
}
