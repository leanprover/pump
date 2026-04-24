use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::{analyze_global, analyze_version};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobQueryDataV0 {
    AnalyzeGlobal { input: analyze_global::Input },
    AnalyzeVersion { input: analyze_version::Input },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JobQueryV0 {
    #[serde(flatten)]
    pub data: JobQueryDataV0,

    #[serde(default)]
    pub force_rerun: bool,
    pub force_rerun_if_older_than_seconds: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobQuery {
    V0(JobQueryV0),
}

impl From<JobQueryV0> for JobQuery {
    fn from(value: JobQueryV0) -> Self {
        Self::V0(value)
    }
}

impl From<JobQuery> for JobQueryV0 {
    fn from(value: JobQuery) -> Self {
        match value {
            JobQuery::V0(job) => job,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JobStatusV0 {
    #[serde(flatten)]
    pub data: JobQueryDataV0,

    pub queued: Timestamp,
    pub started: Option<Timestamp>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum JobStatus {
    V0(JobStatusV0),
}

impl From<JobStatusV0> for JobStatus {
    fn from(value: JobStatusV0) -> Self {
        Self::V0(value)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobResultDataV0 {
    AnalyzeGlobal {
        input: analyze_global::Input,
        output: Option<analyze_global::Output>,
    },
    AnalyzeVersion {
        input: analyze_version::Input,
        output: Option<analyze_version::Output>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JobResultV0 {
    #[serde(flatten)]
    pub data: JobResultDataV0,

    pub queued: Timestamp,
    pub started: Timestamp,
    pub finished: Timestamp,
    pub exit_code: u8,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum JobResult {
    V0(JobResultV0),
}

impl From<JobResultV0> for JobResult {
    fn from(value: JobResultV0) -> Self {
        Self::V0(value)
    }
}
