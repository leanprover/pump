mod query;
mod queue;

use axum::Router;
use axum::routing::{get, post};
use tokio::net::TcpListener;

use crate::AppState;

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/query", post(query::query))
        .route("/queue", get(queue::queue))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:5800").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
