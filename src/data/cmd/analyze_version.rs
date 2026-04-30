use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::data::common::{SourceV0, TimesV0};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputV0 {
    pub source: SourceV0,
    pub sha: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum Input {
    V0(InputV0),
}

impl From<InputV0> for Input {
    fn from(value: InputV0) -> Self {
        Self::V0(value)
    }
}

impl From<Input> for InputV0 {
    fn from(value: Input) -> Self {
        match value {
            Input::V0(input) => input,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LakeV0 {
    pub license: String,
    pub license_files: Vec<String>,
    pub platform_independent: Option<bool>,
    pub readme_file: String,
    pub version: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OutputV0 {
    pub sha: String,
    pub toolchain: Option<String>,
    pub active_toolchain: Option<String>,
    pub manifest: Option<Value>,
    pub lake: Option<LakeV0>,
    pub check_build: Option<bool>,
    pub check_test: Option<bool>,
    pub check_lint: Option<bool>,

    #[serde(flatten)]
    pub times: TimesV0,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum Output {
    V0(OutputV0),
}
