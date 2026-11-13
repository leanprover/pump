use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    sync::{Arc, Mutex},
};

use jiff::{SignedDuration, Timestamp};
use log::{error, info};

use crate::{
    AppState, TICK_INTERVAL,
    data::{JobInput, JobQuery, Source},
    impeller::{self, Log, threads_for_input, version_for_input},
    server::data::{Batch, JobStatus},
    storage::{
        BatchState, BatchStore,
        data::{BatchV0, JobResultV0},
    },
};

/// A job that needs to run.
#[derive(Clone)]
pub struct Pending {
    pub input: JobInput,

    /// Creation time of the earliest active batch that still wants this job.
    ///
    /// A batch that is satisfied with results currently available in the job
    /// storage doesn't count.
    pub queued: Timestamp,
}

/// A job currently being executed.
#[derive(Clone)]
pub struct Running {
    pub input: JobInput,
    pub queued: Timestamp,
    pub started: Timestamp,
    pub threads: usize,
    pub log: Arc<Mutex<Log>>,
}

/// All in-progress runs, by job id.
#[derive(Default)]
pub struct Runs(pub HashMap<String, Running>);

/// Whether a run started at `started` is recent enough for a batch.
fn fresh_enough(started: Timestamp, max_age_seconds: Option<u64>, created: Timestamp) -> bool {
    let Some(max_age) = max_age_seconds else {
        return true;
    };
    let max_age = SignedDuration::from_secs(i64::try_from(max_age).unwrap_or(i64::MAX));
    match started.checked_add(max_age) {
        Ok(deadline) => deadline >= created,
        Err(_) => true, // Overflow, so effectively no limit
    }
}

/// Whether the current impeller can produce a result satisfying the query.
pub fn check_satisfiable(state: &AppState, query: &JobQuery) -> Result<(), String> {
    if let Some(min) = query.min_version {
        let current = version_for_input(state, &query.input);
        if min > current {
            return Err(format!(
                "min_version {min} can't be satisfied by current output version {current}"
            ));
        }
    }

    if let Some(min) = query.min_app_version {
        let current = state.config.impeller.version;
        if min > current {
            return Err(format!(
                "min_app_version {min} can't be satisfied by current app version {current}"
            ));
        }
    }

    Ok(())
}

/// Whether a stored result satisfies a query of a batch.
///
/// Minimum versions are only checked against the current impeller when the
/// batch is created. If impeller changed since then (e.g. because pump was
/// restarted and impeller downgraded), a minimum version may no longer be
/// satisfiable, so it is lowered to the current version. Otherwise, the job
/// would be rerun forever.
fn result_satisfies(
    state: &AppState,
    result: &JobResultV0,
    query: &JobQuery,
    created: Timestamp,
) -> bool {
    let min_version = query
        .min_version
        .map(|it| it.min(version_for_input(state, &query.input)));

    let min_app_version = query
        .min_app_version
        .map(|it| it.min(state.config.impeller.version));

    fresh_enough(result.started, query.max_age_seconds, created)
        && min_version.is_none_or(|it| it <= result.version)
        && min_app_version.is_none_or(|it| it <= result.app_version)
}

/// Versions don't need to be checked since in-progress runs always use the
/// current impeller.
fn run_satisfies(run: &Running, query: &JobQuery, created: Timestamp) -> bool {
    fresh_enough(run.started, query.max_age_seconds, created)
}

#[derive(Default)]
struct Queue {
    /// Jobs that need to run, by their id.
    ///
    /// A job may appear in both `running` and `pending` if the current run
    /// doesn't satisfy a batch's query, e.g. because the age requirements are
    /// too strict.
    pending: HashMap<String, Pending>,

    /// Ids of batches whose queries are all satisfied by stored results.
    finished_batches: HashSet<String>,
}

/// Determine the current state of the queue.
fn analyze_queue(state: &AppState, runs: &Runs, batches: &[BatchV0]) -> anyhow::Result<Queue> {
    // Caching job results so we don't get an inconsistent queue state.
    let mut results = HashMap::new();

    let mut pending = HashMap::new();
    let mut finished_batches = HashSet::new();

    for batch in batches {
        let mut finished = true;

        for query in &batch.jobs {
            let id = query.input.id();

            // Finished jobs that satisfy the query don't need to be re-queued.
            if let Some(result) = match results.entry(id.clone()) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(state.jobs.get(&id)?),
            } && result_satisfies(state, result, query, batch.created)
            {
                continue;
            }

            finished = false;

            // Running jobs that satisfy the query also don't need to be queued.
            if let Some(run) = runs.0.get(&id)
                && run_satisfies(run, query, batch.created)
            {
                continue;
            }

            // Query not satisfied by existing job, so we need to queue it.
            let pending_job = pending.entry(id.clone()).or_insert_with(|| Pending {
                input: query.input.clone(),
                queued: batch.created,
            });
            pending_job.queued = pending_job.queued.min(batch.created);
        }

        if finished {
            finished_batches.insert(batch.id.clone());
        }
    }

    Ok(Queue {
        pending,
        finished_batches,
    })
}

/// Running and pending jobs, sorted by when they were queued.
///
/// If a job is both currently running and also pending, only the running job
/// will be listed.
pub fn get_queue(state: &AppState, store: &BatchStore) -> anyhow::Result<Vec<JobStatus>> {
    let runs = state.runs.lock().unwrap();
    let batches = &store.list(BatchState::Running)?;
    let queue = analyze_queue(state, &runs, batches)?;

    let mut result = vec![];

    for run in runs.0.values() {
        result.push(JobStatus {
            id: run.input.id(),
            input: run.input.clone(),
            queued: Some(run.queued),
            started: Some(run.started),
            finished: None,
        });
    }

    for pending in queue.pending.values() {
        if runs.0.contains_key(&pending.input.id()) {
            continue;
        }
        result.push(JobStatus {
            id: pending.input.id(),
            input: pending.input.clone(),
            queued: Some(pending.queued),
            started: None,
            finished: None,
        });
    }

    result.sort_by(|a, b| (a.queued, &a.id).cmp(&(b.queued, &b.id)));

    Ok(result)
}

/// The API view of a batch.
pub fn get_batch_info(
    state: &AppState,
    store: &BatchStore,
    batch_state: BatchState,
    batch: &BatchV0,
) -> anyhow::Result<Batch> {
    let running = batch_state == BatchState::Running;

    let batches = store.list(BatchState::Running)?;
    let runs = state.runs.lock().unwrap();
    let queue = analyze_queue(state, &runs, &batches)?;

    let mut jobs = HashMap::new();
    for query in &batch.jobs {
        let id = query.input.id();

        let mut queued = None;
        let mut started = None;
        let mut finished = None;

        if let Some(job) = state.jobs.get(&id)?
            && result_satisfies(state, &job, query, batch.created)
        {
            queued = Some(job.queued);
            started = Some(job.started);
            finished = Some(job.finished);
        } else if let Some(run) = runs.0.get(&id)
            && run_satisfies(run, query, batch.created)
        {
            queued = Some(run.queued);
            started = Some(run.started);
        } else if let Some(pending) = queue.pending.get(&id) {
            queued = Some(pending.queued);
        }

        jobs.insert(
            id.clone(),
            JobStatus {
                id,
                input: query.input.clone(),
                queued,
                started,
                finished,
            },
        );
    }

    Ok(Batch {
        id: batch.id.clone(),
        created: batch.created,
        running,
        callback_url: batch.callback_url.clone(),
        jobs,
    })
}

fn start_run(state: &AppState, runs: &mut Runs, job: Pending, threads: usize) {
    let id = job.input.id();
    info!("Starting {id}");

    let started = Timestamp::now();
    let log = Arc::new(Mutex::new(Log::new(state.config.impeller.max_log_bytes)));
    let run = Running {
        input: job.input.clone(),
        queued: job.queued,
        started,
        threads,
        log: log.clone(),
    };
    runs.0.insert(id.clone(), run);

    let state = state.clone();
    tokio::spawn(async move {
        let result = impeller::run(&state, job.input, job.queued, started, threads, &log).await;
        info!("Finished {id} with exit code {}", result.exit_code);

        // Store the result before removing the run so the job is never
        // considered missing in between.
        if let Err(e) = state.jobs.put(result) {
            error!("Failed to store result of {id}: {e:#}");
        }
        state.runs.lock().unwrap().0.remove(&id);
        state.wake_scheduler.notify_one();
    });
}

fn tick(state: &AppState) -> anyhow::Result<()> {
    let mut store = state.batches.lock().unwrap();
    let mut runs = state.runs.lock().unwrap();

    let batches = store.list(BatchState::Running)?;
    let queue = analyze_queue(state, &runs, &batches)?;

    for id in &queue.finished_batches {
        info!("Batch {id} is fulfilled");
        store.advance(id, BatchState::Running, BatchState::Stopping)?;
        state.wake_callbacks.notify_one();
    }

    let total_threads = state.config.queue.threads_total;
    let mut occupied_threads = runs.0.values().map(|run| run.threads).sum::<usize>();
    let mut occupied_sources = (runs.0.values())
        .map(|run| run.input.source().clone())
        .collect::<HashSet<Source>>();

    let mut pending = queue.pending.into_iter().collect::<Vec<_>>();
    pending.sort_unstable_by(|(a_id, a), (b_id, b)| (a.queued, a_id).cmp(&(b.queued, b_id)));
    for (id, job) in pending {
        // This check is redundant because of the sources check, but of course a
        // job can't be started again while it's already running. Keeping the
        // check for clarity/future-proofing.
        if runs.0.contains_key(&id) {
            continue;
        }

        // Jobs may be postponed if another job for the same source is already
        // running because they would use the same subdirectory in the repo dir.
        if occupied_sources.contains(job.input.source()) {
            continue;
        }

        // Jobs may be postponed if there aren't enough free threads, to
        // increase the throughput at the cost of a bit of latency.
        let threads = threads_for_input(state, &job.input).min(total_threads);
        if occupied_threads + threads > total_threads {
            continue;
        }

        occupied_threads += threads;
        occupied_sources.insert(job.input.source().clone());
        start_run(state, &mut runs, job, threads);
    }

    Ok(())
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    loop {
        if let Err(e) = tick(&state) {
            error!("Scheduler failed: {e:#}");
        }

        tokio::select! {
            _ = state.wake_scheduler.notified() => {}
            _ = tokio::time::sleep(TICK_INTERVAL) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn freshness() {
        let created = ts("2026-01-01T12:00:00Z");
        let before = ts("2026-01-01T11:59:00Z");
        let after = ts("2026-01-01T12:00:01Z");

        assert!(fresh_enough(before, None, created));
        assert!(fresh_enough(created, Some(0), created));
        assert!(fresh_enough(after, Some(0), created));
        assert!(!fresh_enough(before, Some(0), created));
        assert!(!fresh_enough(before, Some(59), created));
        assert!(fresh_enough(before, Some(60), created));
        assert!(fresh_enough(before, Some(u64::MAX), created));
    }
}
