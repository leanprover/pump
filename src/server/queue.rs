use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::data::job::JobStatus;
use crate::server::ServerState;

#[derive(Serialize)]
pub struct QueueReply {
    pending: Vec<JobStatus>,
}

pub async fn queue(State(state): State<ServerState>) -> Json<QueueReply> {
    let pending = state.queue.lock().unwrap().pending();
    Json(QueueReply { pending })
}
