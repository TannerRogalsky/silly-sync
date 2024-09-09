use crate::{oauth::AuthRedirect, SessionStore};
use async_session::SessionStore as _;
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    RequestPartsExt as _,
};
use axum_extra::{headers, typed_header::TypedHeaderRejectionReason, TypedHeader};
use http::{header, request::Parts};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub avatar: Option<String>,
    pub username: String,
    pub discriminator: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for User
where
    SessionStore: FromRef<S>,
    S: Send + Sync,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = AuthRedirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let store = SessionStore::from_ref(state);

        let cookies = parts
            .extract::<TypedHeader<headers::Cookie>>()
            .await
            .map_err(|e| match *e.name() {
                header::COOKIE => match e.reason() {
                    TypedHeaderRejectionReason::Missing => AuthRedirect,
                    _ => panic!("unexpected error getting Cookie header(s): {e}"),
                },
                _ => panic!("unexpected error getting cookies: {e}"),
            })?;
        let session_cookie = cookies.get(crate::COOKIE_NAME).ok_or(AuthRedirect)?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .unwrap()
            .ok_or(AuthRedirect)?;

        let user = session.get::<User>("user").ok_or(AuthRedirect)?;

        Ok(user)
    }
}
