use std::{fs, path::Path};

use anyhow::Context;
use serde::Deserialize;
use serde_json::{Map, Value};

mod default {
    pub(super) fn impeller_cmd() -> String {
        "impeller".to_string()
    }

    pub(super) fn queue_threads_total() -> usize {
        num_cpus::get()
    }

    pub(super) fn queue_threads_build_version() -> usize {
        8
    }
}

#[derive(Deserialize)]
pub struct Impeller {
    #[serde(default = "default::impeller_cmd")]
    pub cmd: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub args_analyze_global: Vec<String>,

    #[serde(default)]
    pub args_analyze_version: Vec<String>,

    #[serde(default)]
    pub args_build_version: Vec<String>,
}

impl Default for Impeller {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

#[derive(Deserialize)]
pub struct Queue {
    #[serde(default = "default::queue_threads_total")]
    pub threads_total: usize,

    #[serde(default = "default::queue_threads_build_version")]
    pub threads_build_version: usize,
}

impl Default for Queue {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub impeller: Impeller,

    #[serde(default)]
    pub queue: Queue,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config =
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }
}
