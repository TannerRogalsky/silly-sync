use crate::user::User;
use anyhow::Context;
use async_session::SessionStore as _;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::{headers, TypedHeader};
use http::HeaderMap;
pub use oauth2::basic::BasicClient;
use oauth2::{
    reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};

impl axum::extract::FromRef<crate::AppState> for BasicClient {
    fn from_ref(state: &crate::AppState) -> Self {
        state.oauth_client.clone()
    }
}

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        Redirect::temporary("/auth/discord").into_response()
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct AuthRequest {
    code: String,
    #[allow(unused)]
    state: String,
}

pub fn oauth_client() -> Result<BasicClient, crate::AppError> {
    // Environment variables (* = required):
    // *"CLIENT_ID"     "REPLACE_ME";
    // *"CLIENT_SECRET" "REPLACE_ME";
    //  "REDIRECT_URL"  "http://127.0.0.1:3000/auth/authorized";
    //  "AUTH_URL"      "https://discord.com/api/oauth2/authorize?response_type=code";
    //  "TOKEN_URL"     "https://discord.com/api/oauth2/token";

    use std::env;

    let client_id = env::var("CLIENT_ID").context("Missing CLIENT_ID!")?;
    let client_secret = env::var("CLIENT_SECRET").context("Missing CLIENT_SECRET!")?;
    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000/auth/authorized".to_string());

    let auth_url = env::var("AUTH_URL").unwrap_or_else(|_| {
        "https://discord.com/api/oauth2/authorize?response_type=code".to_string()
    });

    let token_url = env::var("TOKEN_URL")
        .unwrap_or_else(|_| "https://discord.com/api/oauth2/token".to_string());

    Ok(BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url).context("failed to create new authorization server URL")?,
        Some(TokenUrl::new(token_url).context("failed to create new token endpoint URL")?),
    )
    .set_redirect_uri(
        RedirectUrl::new(redirect_url).context("failed to create new redirection URL")?,
    ))
}

pub async fn discord_auth(State(client): State<BasicClient>) -> impl IntoResponse {
    // TODO: this example currently doesn't validate the CSRF token during login attempts. That
    // makes it vulnerable to cross-site request forgery. If you copy code from this example make
    // sure to add a check for the CSRF token.
    //
    // Issue for adding check to this example https://github.com/tokio-rs/axum/issues/2511
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();

    // Redirect to Discord's oauth service
    Redirect::to(auth_url.as_ref())
}

pub async fn logout(
    State(store): State<crate::SessionStore>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> Result<impl IntoResponse, crate::AppError> {
    let cookie = cookies
        .get(crate::COOKIE_NAME)
        .context("unexpected error getting cookie name")?;

    let session = match store
        .load_session(cookie.to_string())
        .await
        .context("failed to load session")?
    {
        Some(s) => s,
        // No session active, just redirect
        None => return Ok(Redirect::to("/")),
    };

    store
        .destroy_session(session)
        .await
        .context("failed to destroy session")?;

    Ok(Redirect::to("/"))
}

pub async fn login_authorized(
    // Query(query): Query<AuthRequest>,
    State(store): State<crate::SessionStore>,
    State(oauth_client): State<BasicClient>,
) -> Result<impl IntoResponse, crate::AppError> {
    // Get an auth token
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(String::new()))
        // .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(async_http_client)
        .await
        .context("failed in sending request request to authorization server")?;

    // Fetch user data from discord
    let client = reqwest::Client::new();
    let user_data: User = client
        // https://discord.com/developers/docs/resources/user#get-current-user
        .get("https://discordapp.com/api/users/@me")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .context("failed in sending request to target Url")?
        .json::<User>()
        .await
        .context("failed to deserialize response as JSON")?;

    // Create a new session filled with user data
    let mut session = async_session::Session::new();
    session
        .insert("user", &user_data)
        .context("failed in inserting serialized value into session")?;

    // Store session and get corresponding cookie
    let cookie = store
        .store_session(session)
        .await
        .context("failed to store session")?
        .context("unexpected error retrieving cookie value")?;

    // Build the cookie
    let cookie = format!("{}={cookie}; SameSite=Lax; Path=/", crate::COOKIE_NAME);

    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::SET_COOKIE,
        cookie.parse().context("failed to parse cookie")?,
    );

    Ok((headers, Redirect::to("/")))
}
