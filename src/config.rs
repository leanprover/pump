use std::{
    collections::HashMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::Deserialize;
use serde_json::{Map, Value};

mod default {
    pub(super) fn server_addr() -> String {
        "[::]:5800".to_string()
    }

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
pub struct Server {
    #[serde(default = "default::server_addr")]
    pub address: String,
}

impl Default for Server {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
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
pub struct Client {
    pub password: Option<String>,
    pub password_file: Option<PathBuf>,
}

impl Default for Client {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: Server,

    #[serde(default)]
    pub impeller: Impeller,

    #[serde(default)]
    pub queue: Queue,

    #[serde(default)]
    pub clients: HashMap<String, Client>,
}

impl Default for Config {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display()))?,
        };
        let config =
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    pub fn resolve(self) -> anyhow::Result<ResolvedConfig> {
        let mut clients = HashMap::new();
        for (username, client) in self.clients {
            if let Some(password) = client.password {
                clients.insert(username, password);
            } else if let Some(path) = client.password_file {
                let password = fs::read_to_string(path)?.trim().to_string();
                clients.insert(username, password);
            }
        }

        Ok(ResolvedConfig {
            server: self.server,
            impeller: self.impeller,
            queue: self.queue,
            clients,
        })
    }
}

pub struct ResolvedConfig {
    pub server: Server,
    pub impeller: Impeller,
    pub queue: Queue,
    pub clients: HashMap<String, String>,
}
