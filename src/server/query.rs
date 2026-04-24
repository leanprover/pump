use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    data::job::{JobQuery, JobQueryV0, JobResult, JobStatus},
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
        let job: JobQueryV0 = job.into();
        let job_id = job.id();

        if let Some(status) = queue.status_for(&job_id) {
            pending.insert(key, status);
            continue;
        }

        if let Some(result) = state.cache.get(&job_id) {
            let rerun = job.force_rerun
                || job
                    .force_rerun_if_older_than_seconds
                    .map(|seconds| result.started() < Timestamp::now() - seconds.seconds())
                    .unwrap_or(false);
            if !rerun {
                completed.insert(key, result);
                continue;
            }
        }

        let status = queue.enqueue(job);
        pending.insert(key, status);
    }

    Json(QueryReply { completed, pending })
}
