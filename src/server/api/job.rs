use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    server::{api::Error, auth::AuthedClient, data::JobResult},
};

pub async fn get_by_id(
    _client: AuthedClient,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobResult>, Error> {
    let result = state.jobs.get(&id)?.ok_or(Error::NotFound)?;
    Ok(Json(JobResult {
        id,
        input: result.input,
        output: result.output,
        version: result.version,
        app_version: result.app_version,
        queued: result.queued,
        started: result.started,
        finished: result.finished,
        exit_code: result.exit_code,
        log: result.log,
    }))
}
