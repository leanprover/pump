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
        let query: JobQueryV0 = job.into();
        let input = query.input();

        if let Some(status) = queue.status_for(&input) {
            pending.insert(key, status);
            continue;
        }

        if let Some(result) = state.cache.get(&input) {
            let rerun_because_too_old = query
                .force_rerun_if_older_than_seconds
                .map(|seconds| result.finished() < Timestamp::now() - seconds.seconds())
                .unwrap_or(false);

            let rerun_because_failed =
                query.force_rerun_if_nonzero_status && result.exit_code() != 0;

            let rerun = rerun_because_too_old || rerun_because_failed;
            if !rerun {
                completed.insert(key, result);
                continue;
            }
        }

        let status = queue.enqueue(query);
        pending.insert(key, status);
    }

    Json(QueryReply { completed, pending })
}
