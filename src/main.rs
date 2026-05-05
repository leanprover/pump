mod cache;
mod config;
mod data;
mod impeller;
mod queue;
mod server;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use clap::Parser;
use log::info;
use tokio::select;

use crate::cache::Cache;
use crate::config::{Config, ResolvedConfig};
use crate::queue::Queue;

#[derive(Clone)]
struct AppState {
    pub config: &'static ResolvedConfig,
    pub repos_dir: &'static Path,
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
    repos_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let config = Box::leak(Box::new(Config::load(&args.config)?.resolve()?));
    info!("Threads available: {}", config.queue.threads_total);

    fs::create_dir_all(&args.cache_dir)?;
    fs::create_dir_all(&args.repos_dir)?;

    let repos_dir = Box::leak(Box::new(args.repos_dir));
    let cache = Cache::new(args.cache_dir);
    let queue = Queue::new();

    cache.fix_entries()?;

    let state = AppState {
        config,
        repos_dir,
        cache: Arc::new(cache),
        queue: Arc::new(Mutex::new(queue)),
    };

    let state_queue = state.clone();
    let state_server = state;

    select! {
        res = queue::run(state_queue) => res,
        res = server::run(state_server) => res,
    }
}
