mod cache;
mod config;
mod data;
mod somehow;

use std::path::PathBuf;

use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::config::Config;
use crate::data::job::{JobQuery, JobResult, JobStatus, JobStatusV0};

#[derive(Deserialize)]
struct QueryRequest {
    jobs: Vec<JobQuery>,
}

#[derive(Serialize)]
struct QueryReply {
    completed: Vec<JobResult>,
    pending: Vec<JobStatus>,
}

#[derive(Parser)]
struct Args {
    #[arg(long, short, default_value = "config.toml")]
    config: PathBuf,

    #[arg(long, default_value = "results")]
    result_dir: PathBuf,

    #[arg(long, default_value = "repos")]
    repo_dir: PathBuf,
}

async fn query(Json(body): Json<QueryRequest>) -> Json<QueryReply> {
    let pending: Vec<JobStatus> = body
        .jobs
        .into_iter()
        .map(|job| {
            JobStatus::V0(JobStatusV0 {
                data: job.query_data(),
                queued: Timestamp::now(),
                started: None,
            })
        })
        .collect();

    Json(QueryReply {
        completed: vec![],
        pending,
    })
}

#[derive(Serialize)]
struct QueueReply {
    pending: Vec<JobStatus>,
}

async fn queue() -> Json<QueueReply> {
    Json(QueueReply { pending: vec![] })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _config = Config::load(&args.config)?;

    let app = Router::new()
        .route("/query", post(query))
        .route("/queue", get(queue));

    let listener = TcpListener::bind("127.0.0.1:5800").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
