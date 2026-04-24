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

use crate::cache::Cache;
use crate::config::Config;
use crate::queue::Queue;
use crate::server::ServerState;

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
    let _config = Config::load(&args.config)?;

    let state = ServerState {
        cache: Arc::new(Cache::new(args.cache_dir)),
        queue: Arc::new(Mutex::new(Queue::new())),
    };

    let queue_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            queue_state.queue.lock().unwrap().update(&queue_state.cache);
        }
    });

    server::run(state).await
}
