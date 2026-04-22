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
}

#[derive(Deserialize)]
pub struct ImpellerConfig {
    #[serde(default = "default::cmd_analyze_global")]
    pub cmd_analyze_global: Vec<String>,

    #[serde(default = "default::cmd_analyze_version")]
    pub cmd_analyze_version: Vec<String>,
}

impl Default for ImpellerConfig {
    fn default() -> Self {
        Self {
            cmd_analyze_global: default::cmd_analyze_global(),
            cmd_analyze_version: default::cmd_analyze_version(),
        }
    }
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub impeller: ImpellerConfig,
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
