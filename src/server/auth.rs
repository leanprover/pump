use axum::{
    extract::FromRequestParts,
    http::{HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::headers::{Authorization, HeaderMapExt, authorization::Basic};

use crate::AppState;

/// A client that authenticated via HTTP basic auth against the `clients`
/// section of the config.
pub struct AuthedClient {
    pub name: String,
}

pub struct Unauthorized;

impl IntoResponse for Unauthorized {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic"))],
        )
            .into_response()
    }
}

impl FromRequestParts<AppState> for AuthedClient {
    type Rejection = Unauthorized;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .typed_get::<Authorization<Basic>>()
            .ok_or(Unauthorized)?;

        state
            .config
            .clients
            .get(auth.username())
            .filter(|pw| *pw == auth.password())
            .ok_or(Unauthorized)?;

        Ok(Self {
            name: auth.username().to_string(),
        })
    }
}
