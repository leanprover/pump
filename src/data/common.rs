use jiff::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceV0 {
    Github { owner: String, repo: String },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TimesV0 {
    pub started: Timestamp,
    pub finished: Timestamp,
}
