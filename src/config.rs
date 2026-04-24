use std::{fs, path::Path};

use anyhow::Context;
use serde::Deserialize;

mod default {
    pub(super) fn cmd_analyze_global() -> Vec<String> {
        vec!["impeller-analyze-global".to_string()]
    }

    pub(super) fn cmd_analyze_version() -> Vec<String> {
        vec!["impeller-analyze-version".to_string()]
    }

    pub(super) fn bubblewrap() -> bool {
        true
    }

    pub(super) fn bubblewrap_nixos() -> bool {
        false
    }

    pub(super) fn threads_total() -> usize {
        num_cpus::get()
    }

    pub(super) fn threads_analyze_global() -> usize {
        1
    }

    pub(super) fn threads_analyze_version() -> usize {
        1
    }
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default::cmd_analyze_global")]
    pub cmd_analyze_global: Vec<String>,

    #[serde(default = "default::cmd_analyze_version")]
    pub cmd_analyze_version: Vec<String>,

    #[serde(default = "default::bubblewrap")]
    pub bubblewrap: bool,

    #[serde(default = "default::bubblewrap_nixos")]
    pub bubblewrap_nixos: bool,

    #[serde(default = "default::threads_total")]
    pub threads_total: usize,

    #[serde(default = "default::threads_analyze_global")]
    pub threads_analyze_global: usize,

    #[serde(default = "default::threads_analyze_version")]
    pub threads_analyze_version: usize,
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
