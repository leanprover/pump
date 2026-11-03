mod callback;
mod config;
mod data;
mod impeller;
mod runner;
mod server;
mod storage;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::Parser;
use log::info;
use tokio::{select, sync::Notify};

use crate::{
    config::{Config, ConfigFile},
    runner::Runs,
    storage::{BatchStore, JobStore},
};

/// How often the scheduler and callbacks run even if nothing woke them up.
const TICK_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct AppState {
    pub config: &'static Config,
    pub repos_dir: &'static Path,
    pub jobs: Arc<JobStore>,
    pub batches: Arc<Mutex<BatchStore>>,
    pub runs: Arc<Mutex<Runs>>,
    pub wake_scheduler: Arc<Notify>,
    pub wake_callbacks: Arc<Notify>,
}

#[derive(Parser)]
struct Args {
    #[arg(long, short, default_value = "config.toml")]
    config: PathBuf,

    #[arg(long, default_value = "batch")]
    batch_dir: PathBuf,

    #[arg(long, default_value = "job")]
    job_dir: PathBuf,

    #[arg(long, default_value = "repos")]
    repos_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let config = ConfigFile::load(&args.config)?.resolve()?;
    let config = Box::leak(Box::new(config));
    info!("Impeller version: {}", config.impeller.version);
    info!(
        "Impeller version (analyze-global): {}",
        config.impeller.version_analyze_global
    );
    info!(
        "Impeller version (analyze-version): {}",
        config.impeller.version_analyze_version
    );
    info!(
        "Impeller version (build-version): {}",
        config.impeller.version_build_version
    );
    info!("Threads available: {}", config.queue.threads_total);

    fs::create_dir_all(&args.repos_dir)?;

    let repos_dir = Box::leak(Box::new(args.repos_dir));
    let jobs = JobStore::new(args.job_dir)?;
    let batches = BatchStore::new(args.batch_dir)?;

    let state = AppState {
        config,
        repos_dir,
        jobs: Arc::new(jobs),
        batches: Arc::new(Mutex::new(batches)),
        runs: Arc::new(Mutex::new(Runs::default())),
        wake_scheduler: Arc::new(Notify::new()),
        wake_callbacks: Arc::new(Notify::new()),
    };

    select! {
        res = server::run(state.clone()) => res,
        res = runner::run(state.clone()) => res,
        res = callback::run(state) => res,
    }
}
