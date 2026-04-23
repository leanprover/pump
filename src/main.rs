mod config;
mod data;
mod somehow;
mod types;

use std::path::PathBuf;

use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use tokio::net::TcpListener;

use crate::config::Config;
use crate::types::{Job, QueryReply, QueryRequest, QueueReply};

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
    let pending: Vec<Job> = body
        .jobs
        .into_iter()
        .map(|input| Job {
            input,
            running_since: None,
        })
        .collect();

    Json(QueryReply {
        completed: vec![],
        pending,
    })
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
