mod api;
mod auth;
pub mod data;

use axum::Router;
use axum::routing::{delete, get, post};
use log::info;
use tokio::net::TcpListener;

use crate::AppState;

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let address = state.config.server.address.clone();

    let app = Router::new()
        .route("/v1/batch", post(api::batch::post))
        .route("/v1/batch/{id}", get(api::batch::get_by_id))
        .route("/v1/batch/{id}", delete(api::batch::delete_by_id))
        .route("/v1/job/{id}", get(api::job::get_by_id))
        .route("/v1/queue", get(api::queue::get))
        .route("/v1/queue/{id}", get(api::queue::get_by_id))
        .with_state(state);

    info!("Listening on {address}");
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
