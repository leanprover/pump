use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for Error {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}
