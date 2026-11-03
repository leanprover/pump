pub mod batch;
pub mod job;
pub mod queue;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use log::error;

pub enum Error {
    BadRequest(String),
    NotFound,
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::Internal(e) => {
                error!("Internal error: {e:#}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
