use std::collections::HashMap;

use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

use crate::data::{AppVersion, JobInput, Line, OutputVersion};

#[derive(Debug, Clone, Serialize)]
pub struct Batch {
    pub id: String,

    /// When the batch was created.
    pub created: Timestamp,

    /// Whether the batch is still active or has stopped.
    ///
    /// Once stopped, a batch will no longer cause any jobs to be queued or
    /// executed, though already running jobs may still complete. A batch stops
    /// once all job queries have been fulfilled, or when canceled manually.
    pub running: bool,

    /// The URL to which a POST request is sent when the batch stops running.
    ///
    /// If the POST is unsuccessful, it will be retried a limited number of
    /// times.
    pub callback_url: Option<String>,

    /// Status of all jobs, by their job id.
    ///
    /// Job results that don't match their respective query criteria are not
    /// included in this field and instead the queued job's status is used.
    /// However, these job results may still be present in the job store and
    /// returned when queried directly. Thus, some clients may want to retrieve
    /// only jobs from this field whose status indicates they're finished.
    pub jobs: HashMap<String, JobStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobStatus {
    pub id: String,
    pub input: JobInput,

    pub queued: Option<Timestamp>,
    pub started: Option<Timestamp>,
    pub finished: Option<Timestamp>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobResult {
    pub id: String,
    pub input: JobInput,
    pub output: Option<Value>,

    pub version: OutputVersion,
    pub app_version: AppVersion,

    pub queued: Timestamp,
    pub started: Timestamp,
    pub finished: Timestamp,

    pub exit_code: i32,
    pub log: Option<Vec<Line>>,
}

/// An in-progress run, including its log so far.
#[derive(Debug, Clone, Serialize)]
pub struct Run {
    pub id: String,
    pub input: JobInput,

    pub queued: Timestamp,
    pub started: Timestamp,

    /// Index of the first line in `log`, as requested via `from`.
    pub from: usize,
    pub log: Vec<Line>,
}
