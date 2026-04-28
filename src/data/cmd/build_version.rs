use serde::{Deserialize, Serialize};

use crate::data::common::{SourceV0, TimesV0};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputV0 {
    pub source: SourceV0,
    pub sha: String,
    pub build: Option<bool>,
    pub lint: Option<bool>,
    pub test: Option<bool>,
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
pub struct CommandV0 {
    pub success: bool,

    #[serde(flatten)]
    pub times: TimesV0,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OutputV0 {
    pub sha: String,
    pub check_build: Option<bool>,
    pub check_test: Option<bool>,
    pub check_lint: Option<bool>,
    pub build: Option<CommandV0>,
    pub test: Option<CommandV0>,
    pub lint: Option<CommandV0>,

    #[serde(flatten)]
    pub times: TimesV0,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum Output {
    V0(OutputV0),
}
