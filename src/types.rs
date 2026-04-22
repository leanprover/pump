use serde::{Deserialize, Serialize};
use serde_json::Value;

mod default {
    pub(super) fn get_true() -> bool {
        true
    }

    pub(super) fn is_true(v: &bool) -> bool {
        *v
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Source {
    Github { owner: String, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeGlobal {
    pub source: Source,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeVersion {
    pub source: Source,

    #[serde(default = "default::get_true")]
    #[serde(skip_serializing_if = "default::is_true")]
    pub build: bool,

    #[serde(default = "default::get_true")]
    #[serde(skip_serializing_if = "default::is_true")]
    pub test: bool,

    #[serde(default = "default::get_true")]
    #[serde(skip_serializing_if = "default::is_true")]
    pub lint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum InputConfiguration {
    AnalyzeGlobal(AnalyzeGlobal),
    AnalyzeVersion(AnalyzeVersion),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub input: InputConfiguration,
    pub running_since: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub input: InputConfiguration,
    pub output: Value,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub jobs: Vec<InputConfiguration>,
}

#[derive(Debug, Serialize)]
pub struct QueryReply {
    pub completed: Vec<JobResult>,
    pub pending: Vec<Job>,
}

#[derive(Debug, Serialize)]
pub struct QueueReply {
    pub pending: Vec<Job>,
}
