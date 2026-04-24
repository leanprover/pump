use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::AppState;
use crate::data::job::JobStatus;

#[derive(Serialize)]
pub struct QueueReply {
    pending: Vec<JobStatus>,
}

pub async fn queue(State(state): State<AppState>) -> Json<QueueReply> {
    let pending = state.queue.lock().unwrap().pending();
    Json(QueueReply { pending })
}
