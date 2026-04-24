mod query;
mod queue;

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::routing::{get, post};
use tokio::net::TcpListener;

use crate::cache::Cache;
use crate::queue::Queue;

#[derive(Clone)]
pub struct ServerState {
    pub cache: Arc<Cache>,
    pub queue: Arc<Mutex<Queue>>,
}

pub async fn run(state: ServerState) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/query", post(query::query))
        .route("/queue", get(queue::queue))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:5800").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
