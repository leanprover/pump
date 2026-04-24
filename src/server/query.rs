use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    data::job::{JobQuery, JobResult, JobStatus},
};

#[derive(Deserialize)]
pub struct QueryRequest {
    jobs: HashMap<String, JobQuery>,
}

#[derive(Serialize)]
pub struct QueryReply {
    pending: HashMap<String, JobStatus>,
    completed: HashMap<String, JobResult>,
}

pub async fn query(
    State(state): State<AppState>,
    Json(body): Json<QueryRequest>,
) -> Json<QueryReply> {
    let mut queue = state.queue.lock().unwrap();

    let mut pending = HashMap::new();
    let mut completed = HashMap::new();
    for (key, job) in body.jobs {
        let job_id = job.id();

        if let Some(status) = queue.status_for(&job_id) {
            pending.insert(key, status);
            continue;
        }

        if let Some(result) = state.cache.get(&job_id) {
            completed.insert(key, result);
            continue;
        }

        let status = queue.enqueue(job.into());
        pending.insert(key, status);
    }

    Json(QueryReply { completed, pending })
}
