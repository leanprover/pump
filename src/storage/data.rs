use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::data::{AppVersion, JobInput, JobQuery, Line, OutputVersion};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultV0 {
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

impl JobResultV0 {
    pub fn id(&self) -> String {
        self.input.id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "v", rename_all = "snake_case")]
pub enum JobResult {
    V0(JobResultV0),
}

impl JobResult {
    pub fn into_latest(self) -> JobResultV0 {
        match self {
            Self::V0(result) => result,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchV0 {
    pub id: String,
    pub user: String,
    pub created: Timestamp,

    pub callback_url: Option<String>,

    pub jobs: Vec<JobQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "v", rename_all = "snake_case")]
pub enum Batch {
    V0(BatchV0),
}

impl Batch {
    pub fn into_latest(self) -> BatchV0 {
        match self {
            Self::V0(batch) => batch,
        }
    }
}
