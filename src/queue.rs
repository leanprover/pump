use std::{collections::VecDeque, time::Duration};

use jiff::Timestamp;
use tokio::sync::oneshot::{self, error::TryRecvError};

use crate::{
    AppState,
    cache::Cache,
    data::{
        job::{JobQueryV0, JobResultV0, JobStatus, JobStatusV0},
        job_input::JobInput,
    },
};

struct Job {
    input: JobInput,
    queried: Timestamp,
    started: Option<Timestamp>,
    result: Option<oneshot::Receiver<JobResultV0>>,
}

impl Job {
    fn status(&self) -> JobStatus {
        JobStatusV0 {
            data: self.input.clone().into(),
            queued: self.queried,
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
                Ok(()) => true,
                Err(_) => false, // TODO Log error
            },
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Closed) => {
                // The worker task has aborted in some way without providing any
                // result. This shouldn't normally happen, but if it does, we'll
                // just try again.
                // TODO Log warning
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

        let job = Job {
            input,
            queried: Timestamp::now(),
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

    pub fn finish(&mut self, cache: &Cache) {
        self.0.retain_mut(|job| job.finish(cache));
    }
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let mut queue = state.queue.lock().unwrap();
        queue.finish(&state.cache);
        // TODO Start pending jobs
    }
}
