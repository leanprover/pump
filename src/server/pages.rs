//! Human-readable status pages.
//!
//! The pages are static and render themselves client-side using the JSON API.

use axum::{
    http::header,
    response::{Html, IntoResponse, Redirect},
};

/// The root path, which currently just points at the queue.
pub async fn root() -> Redirect {
    Redirect::temporary("/queue")
}

/// Overview of running and queued jobs.
pub async fn queue() -> Html<&'static str> {
    Html(include_str!("pages/queue.html"))
}

/// A single job while queued or running, including its live log.
pub async fn run() -> Html<&'static str> {
    Html(include_str!("pages/run.html"))
}

/// The result of a finished job.
pub async fn job() -> Html<&'static str> {
    Html(include_str!("pages/job.html"))
}

pub async fn common_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("pages/common.css"),
    )
}

pub async fn common_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("pages/common.js"),
    )
}
