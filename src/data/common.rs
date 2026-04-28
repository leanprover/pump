use std::fmt;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceV0 {
    Github { owner: String, repo: String },
}

impl fmt::Debug for SourceV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceV0::Github { owner, repo } => write!(f, "github:{owner}/{repo}"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TimesV0 {
    pub started: Timestamp,
    pub finished: Timestamp,
}
