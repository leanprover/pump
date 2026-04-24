use std::collections::HashMap;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::data::common::SourceV0;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputV0 {
    pub source: SourceV0,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum Input {
    V0(InputV0),
}

impl From<Input> for InputV0 {
    fn from(value: Input) -> Self {
        match value {
            Input::V0(input) => input,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GitV0 {
    pub version_tags: Vec<String>,
    pub tag_shas: HashMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LakeV0 {
    pub description: String,
    pub do_index: bool,
    pub homepage: String,
    pub keywords: Vec<String>,
    pub name: String,
    pub platform_independent: Option<bool>,
    pub version_tags: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GithubV0 {
    pub name: String,
    pub full_name: String,
    pub owner: String,
    pub description: String,
    pub fork: bool,
    pub html_url: String,
    pub clone_url: String,
    pub homepage: String,
    pub forks_count: u64,
    pub stargazers_count: u64,
    pub subscribers_count: u64,
    pub default_branch: String,
    pub topics: Vec<String>,
    pub archived: bool,
    pub disabled: bool,
    pub pushed_at: Timestamp,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub license_spdx_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OutputV0 {
    pub git: GitV0,
    pub lake: Option<LakeV0>,
    pub github: Option<GithubV0>,
    // Timings
    pub started: Timestamp,
    pub finished: Timestamp,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum Output {
    V0(OutputV0),
}
