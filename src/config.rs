use std::{
    collections::HashMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, anyhow, bail};
use jiff::SignedDuration;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::data::{AppVersion, OutputVersion};

pub struct Server {
    pub address: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FServer {
    pub address: Option<String>,
}

impl Default for FServer {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

pub struct Impeller {
    pub cmd: String,
    pub args: Vec<String>,
    pub args_analyze_global: Vec<String>,
    pub args_analyze_version: Vec<String>,
    pub args_build_version: Vec<String>,
    pub version: AppVersion,
    pub version_analyze_global: OutputVersion,
    pub version_analyze_version: OutputVersion,
    pub version_build_version: OutputVersion,

    /// Maximum size of a run's log in bytes.
    ///
    /// Further output is skipped. This size includes an estimation of pump's
    /// per-line overhead.
    pub max_log_bytes: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FImpeller {
    pub cmd: Option<String>,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub args_analyze_global: Vec<String>,

    #[serde(default)]
    pub args_analyze_version: Vec<String>,

    #[serde(default)]
    pub args_build_version: Vec<String>,

    pub max_log_bytes: Option<usize>,
}

impl Default for FImpeller {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

pub struct Queue {
    pub threads_total: usize,
    pub threads_build_version: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FQueue {
    pub threads_total: Option<usize>,
    pub threads_build_version: Option<usize>,
}

impl Default for FQueue {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

pub struct Callback {
    /// How often a callback is tried before giving up.
    pub max_tries: u32,
    /// Minimum delay between retries.
    pub retry_delay: Duration,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FCallback {
    pub max_tries: Option<u32>,
    pub retry_delay: Option<SignedDuration>,
}

impl Default for FCallback {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FClient {
    pub password: Option<String>,
    pub password_file: Option<PathBuf>,
}

impl Default for FClient {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    #[serde(default)]
    pub server: FServer,

    #[serde(default)]
    pub impeller: FImpeller,

    #[serde(default)]
    pub queue: FQueue,

    #[serde(default)]
    pub callback: FCallback,

    #[serde(default)]
    pub clients: HashMap<String, FClient>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).unwrap()
    }
}

impl ConfigFile {
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

    fn get_app_version(cmd: &str) -> anyhow::Result<AppVersion> {
        let out = Command::new(cmd).arg("--version").output()?;
        let out = String::from_utf8(out.stdout)?;
        out.trim().parse().map_err(|it| anyhow!("{it}"))
    }

    fn get_output_version(cmd: &str, subcmd: &str) -> anyhow::Result<OutputVersion> {
        let out = Command::new(cmd).arg(subcmd).arg("--version").output()?;
        let out = String::from_utf8(out.stdout)?;
        out.trim().parse().map_err(|it| anyhow!("{it}"))
    }

    pub fn resolve(self) -> anyhow::Result<Config> {
        let server = Server {
            address: self
                .server
                .address
                .unwrap_or_else(|| "[::]:5800".to_string()),
        };

        let impeller_cmd = self.impeller.cmd.unwrap_or_else(|| "impeller".to_string());
        let impeller = Impeller {
            cmd: impeller_cmd.clone(),
            args: self.impeller.args,
            args_analyze_global: self.impeller.args_analyze_global,
            args_analyze_version: self.impeller.args_analyze_version,
            args_build_version: self.impeller.args_build_version,
            version: Self::get_app_version(&impeller_cmd)?,
            version_analyze_global: Self::get_output_version(&impeller_cmd, "analyze-global")?,
            version_analyze_version: Self::get_output_version(&impeller_cmd, "analyze-version")?,
            version_build_version: Self::get_output_version(&impeller_cmd, "build-version")?,
            max_log_bytes: self.impeller.max_log_bytes.unwrap_or(16 * 1024 * 1024), // 16 MiB
        };

        let queue = Queue {
            threads_total: self.queue.threads_total.unwrap_or_else(num_cpus::get),
            threads_build_version: self.queue.threads_build_version.unwrap_or(8),
        };

        let callback = Callback {
            max_tries: self.callback.max_tries.unwrap_or(7),
            retry_delay: match self.callback.retry_delay {
                Some(d) => d
                    .try_into()
                    .map_err(|_| anyhow!("callback.retry_delay must not be negative"))?,
                None => Duration::from_secs(10),
            },
        };

        let mut clients = HashMap::new();
        for (username, client) in self.clients {
            let password = match (client.password, client.password_file) {
                (Some(password), None) => password,
                (None, Some(path)) => fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?
                    .trim_end_matches('\n')
                    .to_string(),
                _ => bail!("client {username:?} needs exactly one of password, password_file"),
            };
            clients.insert(username, password);
        }

        Ok(Config {
            server,
            impeller,
            queue,
            callback,
            clients,
        })
    }
}

pub struct Config {
    pub server: Server,
    pub impeller: Impeller,
    pub queue: Queue,
    pub callback: Callback,
    pub clients: HashMap<String, String>,
}
