use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::data::{
    cmd::{analyze_global, analyze_version, build_version},
    common::SourceV0,
    job::{JobQueryDataV0, JobQueryV0, JobResult, JobResultDataV0, JobResultV0},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum JobInput {
    AnalyzeGlobal { input: analyze_global::InputV0 },
    AnalyzeVersion { input: analyze_version::InputV0 },
    BuildVersion { input: build_version::InputV0 },
}

impl JobInput {
    pub fn hash(&self) -> anyhow::Result<String> {
        Ok(Sha256::digest(serde_json::to_string(self)?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }

    pub fn source(&self) -> &SourceV0 {
        match self {
            JobInput::AnalyzeGlobal { input } => &input.source,
            JobInput::AnalyzeVersion { input } => &input.source,
            JobInput::BuildVersion { input } => &input.source,
        }
    }
}

impl From<JobQueryDataV0> for JobInput {
    fn from(value: JobQueryDataV0) -> Self {
        match value {
            JobQueryDataV0::AnalyzeGlobal { input } => Self::AnalyzeGlobal {
                input: input.into(),
            },
            JobQueryDataV0::AnalyzeVersion { input } => Self::AnalyzeVersion {
                input: input.into(),
            },
            JobQueryDataV0::BuildVersion { input } => Self::BuildVersion {
                input: input.into(),
            },
        }
    }
}

impl From<JobInput> for JobQueryDataV0 {
    fn from(value: JobInput) -> Self {
        match value {
            JobInput::AnalyzeGlobal { input } => Self::AnalyzeGlobal {
                input: input.into(),
            },
            JobInput::AnalyzeVersion { input } => Self::AnalyzeVersion {
                input: input.into(),
            },
            JobInput::BuildVersion { input } => Self::BuildVersion {
                input: input.into(),
            },
        }
    }
}

impl JobQueryV0 {
    pub fn input(&self) -> JobInput {
        self.data.clone().into()
    }
}

impl JobResultDataV0 {
    pub fn input(&self) -> JobInput {
        match self {
            JobResultDataV0::AnalyzeGlobal { input, .. } => JobInput::AnalyzeGlobal {
                input: input.clone().into(),
            },
            JobResultDataV0::AnalyzeVersion { input, .. } => JobInput::AnalyzeVersion {
                input: input.clone().into(),
            },
            JobResultDataV0::BuildVersion { input, .. } => JobInput::BuildVersion {
                input: input.clone().into(),
            },
        }
    }
}

impl JobResultV0 {
    pub fn input(&self) -> JobInput {
        self.data.input()
    }
}

impl JobResult {
    pub fn input(&self) -> JobInput {
        match self {
            JobResult::V0(result) => result.input(),
        }
    }
}
