use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
};
use jiff::Timestamp;
use serde::Deserialize;

use crate::{
    AppState,
    data::{AppVersion, JobQuery},
    runner,
    server::{api::Error, auth::AuthedClient, data::Batch},
    storage::{BatchState, data::BatchV0},
};

pub async fn get_by_id(
    _client: AuthedClient,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Batch>, Error> {
    let store = state.batches.lock().unwrap();
    let (batch_state, batch) = store.get(&id)?.ok_or(Error::NotFound)?;
    let info = runner::get_batch_info(&state, &store, batch_state, &batch)?;
    Ok(Json(info))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostRequest {
    pub jobs: Vec<JobQuery>,

    pub max_age_seconds: Option<u64>,
    pub min_app_version: Option<AppVersion>,

    pub callback_url: Option<String>,
}

/// Apply the batch-level defaults and validate the queries.
fn resolve_jobs(state: &AppState, request: &PostRequest) -> Result<Vec<JobQuery>, String> {
    let mut seen = HashSet::new();
    let mut jobs = vec![];
    for (i, query) in request.jobs.iter().enumerate() {
        query
            .input
            .source()
            .validate()
            .map_err(|e| format!("job {i}: {e}"))?;

        let id = query.input.id();
        if !seen.insert(id.clone()) {
            return Err(format!("job {i}: duplicate input (job id {id})"));
        }

        let query = JobQuery {
            input: query.input.clone(),
            max_age_seconds: query.max_age_seconds.or(request.max_age_seconds),
            min_version: query.min_version,
            min_app_version: query.min_app_version.or(request.min_app_version),
        };
        runner::check_satisfiable(state, &query).map_err(|e| format!("job {i}: {e}"))?;
        jobs.push(query);
    }
    Ok(jobs)
}

fn random_id() -> String {
    rand::random::<[u8; 32]>()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub async fn post(
    client: AuthedClient,
    State(state): State<AppState>,
    Json(request): Json<PostRequest>,
) -> Result<Json<Batch>, Error> {
    if let Some(url) = &request.callback_url {
        reqwest::Url::parse(url)
            .map_err(|e| Error::BadRequest(format!("invalid callback_url: {e}")))?;
    }

    let jobs = resolve_jobs(&state, &request).map_err(Error::BadRequest)?;

    let batch = BatchV0 {
        id: random_id(),
        user: client.name,
        created: Timestamp::now(),
        callback_url: request.callback_url,
        jobs,
    };

    let info = {
        let mut store = state.batches.lock().unwrap();
        store.create(BatchState::Running, batch.clone())?;
        runner::get_batch_info(&state, &store, BatchState::Running, &batch)?
    };

    state.wake_scheduler.notify_one();
    Ok(Json(info))
}

/// Cancel a batch. Canceling a batch that has already stopped does nothing.
pub async fn delete_by_id(
    _client: AuthedClient,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Batch>, Error> {
    let info = {
        let mut store = state.batches.lock().unwrap();
        let (mut batch_state, batch) = store.get(&id)?.ok_or(Error::NotFound)?;

        if batch_state == BatchState::Running {
            store.advance(&id, BatchState::Running, BatchState::Stopping)?;
            batch_state = BatchState::Stopping;
        }

        runner::get_batch_info(&state, &store, batch_state, &batch)?
    };

    state.wake_scheduler.notify_one();
    state.wake_callbacks.notify_one();
    Ok(Json(info))
}
