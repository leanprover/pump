use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::data::{AppVersion, OutputVersion, Source};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeGlobalInput {
    pub source: Source,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeVersionInput {
    pub source: Source,
    pub sha: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildVersionInput {
    pub source: Source,
    pub sha: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_toolchain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum JobInput {
    AnalyzeGlobal(AnalyzeGlobalInput),
    AnalyzeVersion(AnalyzeVersionInput),
    BuildVersion(BuildVersionInput),
}

impl JobInput {
    pub fn id(&self) -> String {
        let json = serde_json::to_string(self).expect("input should be serializable");
        Sha256::digest(json)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    pub fn source(&self) -> &Source {
        match self {
            Self::AnalyzeGlobal(input) => &input.source,
            Self::AnalyzeVersion(input) => &input.source,
            Self::BuildVersion(input) => &input.source,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobQuery {
    pub input: JobInput,

    pub max_age_seconds: Option<u64>,
    pub min_version: Option<OutputVersion>,
    pub min_app_version: Option<AppVersion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Source {
        Source::Github {
            owner: "leanprover".to_string(),
            repo: "lean4".to_string(),
        }
    }

    fn sha() -> String {
        "0123456789abcdef0123456789abcdef01234567".to_string()
    }

    /// Job ids are derived from these hashes. They must not change.
    #[test]
    fn stable_hashes() {
        assert_eq!(
            JobInput::AnalyzeGlobal(AnalyzeGlobalInput { source: source() }).id(),
            "7df07e791da2e2707d9824e9f472e0b9f4fe20f3edee5780ee04130239dd1652"
        );

        assert_eq!(
            JobInput::AnalyzeVersion(AnalyzeVersionInput {
                source: source(),
                sha: sha(),
            })
            .id(),
            "0119b6a18f0eb5e1c94fe0b9bd3a3d3b715bd1ccddc8ff87b7fcc8260a457f2b"
        );

        assert_eq!(
            JobInput::BuildVersion(BuildVersionInput {
                source: source(),
                sha: sha(),
                override_toolchain: None,
                build: None,
                lint: None,
                test: None,
            })
            .id(),
            "2d2c5303625b89ccfc9769c2cf7dd48ac0ab6dd9bf2033b7d782816cccda6237"
        );

        assert_eq!(
            JobInput::BuildVersion(BuildVersionInput {
                source: source(),
                sha: sha(),
                override_toolchain: Some("leanprover/lean4:v4.20.0".to_string()),
                build: Some(true),
                lint: Some(false),
                test: Some(true),
            })
            .id(),
            "eec4e451cf10b4d60c0586c2757d199996dfc13791860cd3bacbd8ae7669f1f0"
        );
    }
}
