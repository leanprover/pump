use std::{
    collections::{HashSet, VecDeque},
    time::Duration,
};

use jiff::Timestamp;
use log::{error, info};
use tokio::sync::oneshot::{self, error::TryRecvError};

use crate::{
    AppState,
    cache::Cache,
    data::{
        common::SourceV0,
        job::{JobQueryV0, JobResultV0, JobStatus, JobStatusV0},
        job_input::JobInput,
    },
    impeller,
};

struct Job {
    input: JobInput,
    queued: Timestamp,
    started: Option<Timestamp>,
    result: Option<oneshot::Receiver<JobResultV0>>,
}

impl Job {
    fn status(&self) -> JobStatus {
        JobStatusV0 {
            data: self.input.clone().into(),
            queued: self.queued,
            started: self.started,
        }
        .into()
    }

    fn finish(&mut self, cache: &Cache) -> bool {
        let Some(rx) = &mut self.result else {
            return false;
        };

        match rx.try_recv() {
            Ok(result) => match cache.put(result.into()) {
                Ok(()) => {
                    info!("Finished {:?}", self.input);
                    true
                }
                Err(e) => {
                    error!("Failed to cache result for {:?}: {e}", self.input);
                    false
                }
            },
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Closed) => {
                // The worker task has aborted in some way without providing any
                // result. This shouldn't normally happen, but if it does, we'll
                // just try again.
                log::warn!(
                    "Worker aborted without result for {:?}, re-queuing",
                    self.input
                );
                self.started = None;
                self.result = None;
                false
            }
        }
    }
}

#[derive(Default)]
pub struct Queue(VecDeque<Job>);

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status_for(&self, data: &JobInput) -> Option<JobStatus> {
        self.0
            .iter()
            .find(|job| job.input == *data)
            .map(|job| job.status())
    }

    pub fn enqueue(&mut self, query: JobQueryV0) -> JobStatus {
        let input = query.input();
        if let Some(status) = self.status_for(&input) {
            return status; // Already enqueued
        }

        info!("Enqueued {input:?}");
        let job = Job {
            input,
            queued: Timestamp::now(),
            started: None,
            result: None,
        };
        let status = job.status();
        self.0.push_back(job);
        status
    }

    pub fn pending(&self) -> Vec<JobStatus> {
        self.0.iter().map(|job| job.status()).collect()
    }

    /// Remove finished jobs from the queue.
    pub fn finish(&mut self, cache: &Cache) {
        self.0.retain_mut(|job| job.finish(cache));
    }
}

fn start_job(state: &AppState, job: &mut Job) {
    assert!(job.result.is_none());

    let state = state.clone();
    let input = job.input.clone();
    let queued = job.queued;
    let started = Timestamp::now();

    info!("Started {input:?}");

    let (tx, rx) = oneshot::channel();
    job.started = Some(started);
    job.result = Some(rx);

    tokio::spawn(async move {
        match impeller::run(state, input.clone(), queued, started).await {
            Ok(result) => {
                let _ = tx.send(result);
            }
            Err(e) => {
                error!("Impeller failed for {input:?}: {e}");
            }
        }
    });
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let mut queue = state.queue.lock().unwrap();
        queue.finish(&state.cache);

        let mut active_threads = 0_usize;
        let mut active_sources = HashSet::<SourceV0>::new();
        for job in queue.0.iter() {
            if job.result.is_some() {
                active_threads += impeller::threads_for_input(&state, &job.input);
                active_sources.insert(job.input.source().clone());
            }
        }

        // Jobs need to be executed in queue order, or else a large job with
        // lots of threads may never receive sufficient threads because later
        // small jobs keep taking up some amount of threads.
        for job in queue.0.iter_mut() {
            if job.result.is_some() {
                continue; // Already running
            }

            let threads = impeller::threads_for_input(&state, &job.input);
            if active_threads + threads > state.config.queue.threads_total {
                break; // Not enough threads available
            };

            if active_sources.contains(job.input.source()) {
                break; // Another job is already running for this source
            }

            start_job(&state, job);
        }
    }
}
