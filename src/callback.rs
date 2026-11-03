use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use log::{error, warn};
use reqwest::Client;

use crate::{
    AppState, TICK_INTERVAL, runner,
    server::data as api,
    storage::{BatchState, data::BatchV0},
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

async fn post(client: &Client, url: &str, body: &api::Batch) -> anyhow::Result<()> {
    client
        .post(url)
        .json(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn stop(state: &AppState, id: &str) -> anyhow::Result<()> {
    let mut store = state.batches.lock().unwrap();
    store.advance(id, BatchState::Stopping, BatchState::Stopped)
}

struct Attempt {
    n: u32,
    when: Instant,
}

async fn tick_batch(
    state: &AppState,
    client: &Client,
    attempts: &mut HashMap<String, Attempt>,
    batch: &BatchV0,
) -> anyhow::Result<()> {
    let config = &state.config.callback;

    let Some(url) = &batch.callback_url else {
        return stop(state, &batch.id);
    };

    if let Some(attempt) = attempts.get(&batch.id)
        && Instant::now() < attempt.when + config.retry_delay
    {
        return Ok(());
    }

    let body = {
        let store = state.batches.lock().unwrap();
        runner::get_batch_info(state, &store, BatchState::Stopping, batch)?
    };

    let Err(e) = post(client, url, &body).await else {
        return stop(state, &batch.id);
    };

    let now = Instant::now();
    let entry = attempts
        .entry(batch.id.clone())
        .or_insert_with(|| Attempt { n: 0, when: now });

    entry.n += 1;
    entry.when = now;

    if entry.n >= config.max_tries {
        error!(
            "Callback for batch {} to {url} failed (attempt {}/{}), giving up: {e:#}",
            batch.id, entry.n, config.max_tries
        );
        stop(state, &batch.id)?;
    } else {
        warn!(
            "Callback for batch {} to {url} failed (attempt {}/{}): {e:#}",
            batch.id, entry.n, config.max_tries
        );
    }

    Ok(())
}

async fn tick(
    state: &AppState,
    client: &Client,
    attempts: &mut HashMap<String, Attempt>,
) -> anyhow::Result<()> {
    let batches = state.batches.lock().unwrap().list(BatchState::Stopping)?;

    let batch_ids = batches
        .iter()
        .map(|it| it.id.clone())
        .collect::<HashSet<_>>();
    attempts.retain(|id, _| batch_ids.contains(id));

    for batch in batches {
        if let Err(e) = tick_batch(state, client, attempts, &batch).await {
            error!("Callback failed for batch {}: {e:#}", batch.id);
        }
    }

    Ok(())
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let client = Client::builder().timeout(REQUEST_TIMEOUT).build()?;
    let mut attempts = HashMap::new();

    loop {
        if let Err(e) = tick(&state, &client, &mut attempts).await {
            error!("Callbacks failed: {e:#}");
        }

        tokio::select! {
            _ = state.wake_callbacks.notified() => {}
            _ = tokio::time::sleep(TICK_INTERVAL) => {}
        }
    }
}
