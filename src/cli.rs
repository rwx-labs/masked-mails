use clap::Parser;
use url::Url;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    /// PostgreSQL URL
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,
    /// Enable tracing
    #[arg(long, env = "ENABLE_TRACING", default_value = "false")]
    pub tracing: bool,
    /// OpenID Connect Provider URL
    #[arg(long, env = "AUTH_ISSUER_URL")]
    pub auth_issuer_url: Url,
    /// OAuth Client ID
    #[arg(long, env = "AUTH_CLIENT_ID")]
    pub auth_client_id: String,
    /// OAuth Client Secret
    #[arg(long, env = "AUTH_CLIENT_SECRET")]
    pub auth_client_secret: String,
    /// OAuth Redirect URL
    #[arg(long, env = "AUTH_REDIRECT_URL")]
    pub auth_redirect_url: Url,
}
