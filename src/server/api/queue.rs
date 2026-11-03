use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, runner,
    server::{
        api::Error,
        data::{JobStatus, Run},
    },
};

#[derive(Serialize)]
pub struct GetReply {
    jobs: Vec<JobStatus>,
}

pub async fn get(State(state): State<AppState>) -> Result<Json<GetReply>, Error> {
    let store = state.batches.lock().unwrap();
    let jobs = runner::get_queue(&state, &store)?;
    Ok(Json(GetReply { jobs }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetRunQuery {
    /// Only return log lines starting at this index.
    #[serde(default)]
    from: usize,
}

/// An in-progress run and its log so far.
pub async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<GetRunQuery>,
) -> Result<Json<Run>, Error> {
    let runs = state.runs.lock().unwrap();
    let run = runs.0.get(&id).ok_or(Error::NotFound)?;
    let log = run.log.lock().unwrap();
    Ok(Json(Run {
        id,
        input: run.input.clone(),
        queued: run.queued,
        started: run.started,
        from: query.from,
        log: log.get(query.from..).unwrap_or_default().to_vec(),
    }))
}
