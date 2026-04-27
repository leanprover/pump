mod cache;
mod config;
mod data;
mod queue;
mod server;
mod somehow;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::Parser;
use tokio::select;

use crate::cache::Cache;
use crate::config::Config;
use crate::queue::Queue;

#[derive(Clone)]
struct AppState {
    pub config: &'static Config,
    pub cache: Arc<Cache>,
    pub queue: Arc<Mutex<Queue>>,
}

#[derive(Parser)]
struct Args {
    #[arg(long, short, default_value = "config.toml")]
    config: PathBuf,

    #[arg(long, default_value = "cache")]
    cache_dir: PathBuf,

    #[arg(long, default_value = "repos")]
    repo_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Config::load(&args.config)?;

    let cache = Cache::new(args.cache_dir);
    cache.fix_entries()?;

    let state = AppState {
        config: Box::leak(Box::new(config)),
        cache: Arc::new(cache),
        queue: Arc::new(Mutex::new(Queue::new())),
    };

    let state_queue = state.clone();
    let state_server = state;

    select! {
        res = queue::run(state_queue) => res,
        res = server::run(state_server) => res,
    }
}
