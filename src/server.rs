mod query;
mod queue;

use axum::Router;
use axum::routing::{get, post};
use log::info;
use tokio::net::TcpListener;

use crate::AppState;

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let addr = state.config.server.addr.clone();

    let app = Router::new()
        .route("/query", post(query::query))
        .route("/queue", get(queue::queue))
        .with_state(state);

    info!("Listening on {addr}");
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
