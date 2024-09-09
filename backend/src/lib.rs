mod error;
mod oauth;
// mod session_store;
mod user;

pub use async_session::MemoryStore as SessionStore;
use axum::{extract::FromRef, response::IntoResponse, routing::get, Router};
use error::AppError;
use tower_service::Service;
use user::User;
use worker::*;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    console_error_panic_hook::set_once();

    Ok(router().call(req).await?)
}

static COOKIE_NAME: &str = "SESSION";

#[derive(Clone)]
struct AppState {
    store: SessionStore,
    oauth_client: oauth::BasicClient,
}

impl FromRef<AppState> for SessionStore {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

impl FromRef<AppState> for oauth::BasicClient {
    fn from_ref(state: &AppState) -> Self {
        state.oauth_client.clone()
    }
}

pub fn router() -> Router {
    let store = SessionStore::new();
    let oauth_client = oauth::oauth_client().unwrap();
    let app_state = AppState {
        store,
        oauth_client,
    };

    Router::new()
        .route("/", get(index))
        .route("/auth/discord", get(oauth::discord_auth))
        .route("/auth/authorized", get(oauth::login_authorized))
        .route("/protected", get(protected))
        .route("/logout", get(oauth::logout))
        .with_state(app_state)
}

// Session is optional
async fn index(user: Option<User>) -> impl IntoResponse {
    match user {
        Some(u) => format!(
            "Hey {}! You're logged in!\nYou may now access `/protected`.\nLog out with `/logout`.",
            u.username
        ),
        None => "You're not logged in.\nVisit `/auth/discord` to do so.".to_string(),
    }
}

async fn protected(user: User) -> impl IntoResponse {
    format!("Welcome to the protected area :)\nHere's your info:\n{user:?}")
}
