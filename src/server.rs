mod query;
mod queue;

use axum::Router;
use axum::routing::{get, post};
use log::info;
use tokio::net::TcpListener;

use crate::AppState;

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/query", post(query::query))
        .route("/queue", get(queue::queue))
        .with_state(state);

    let addr = "127.0.0.1:5800";
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
