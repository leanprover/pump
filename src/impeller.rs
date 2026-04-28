use std::{fs, path::PathBuf, process::Command};

use jiff::Timestamp;
use serde::de::DeserializeOwned;
use tempfile::NamedTempFile;

use crate::{
    AppState,
    data::{
        cmd::{analyze_global, analyze_version, build_version},
        common::SourceV0,
        job::{JobResultDataV0, JobResultV0},
        job_input::JobInput,
    },
};

fn repo_dir_for_source(state: &AppState, source: &SourceV0) -> PathBuf {
    match source {
        SourceV0::Github { owner, repo } => state.repos_dir.join("github").join(owner).join(repo),
    }
}

fn url_for_source(source: &SourceV0) -> String {
    match source {
        SourceV0::Github { owner, repo } => {
            format!("https://github.com/{owner}/{repo}.git")
        }
    }
}

pub fn threads_for_input(state: &AppState, input: &JobInput) -> usize {
    match &input {
        JobInput::AnalyzeGlobal { .. } => 1,
        JobInput::AnalyzeVersion { .. } => 1,
        JobInput::BuildVersion { .. } => state.config.queue.threads_build_version,
    }
}

async fn run_command_and_parse_result<T: DeserializeOwned + Send + 'static>(
    state: &AppState,
    source: &SourceV0,
    mut cmd: Command,
) -> anyhow::Result<(i32, T)> {
    let output_file = NamedTempFile::new()?;

    cmd.arg("--repo").arg(repo_dir_for_source(state, source));
    cmd.arg("--url").arg(url_for_source(source));
    cmd.arg("--output").arg(output_file.path());

    let status = tokio::task::spawn_blocking(move || cmd.status()).await??;
    let exit_code = status.code().unwrap_or(-1);

    let json = fs::read_to_string(&output_file)?;
    let result = serde_json::from_str::<T>(&json)?;

    Ok((exit_code, result))
}

async fn run_analyze_global(
    ctx: RunContext,
    input: analyze_global::InputV0,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&ctx.state.config.impeller.cmd);
    cmd.args(&ctx.state.config.impeller.args);
    cmd.args(&ctx.state.config.impeller.args_analyze_global);

    let (exit_code, output) =
        run_command_and_parse_result::<analyze_global::Output>(&ctx.state, &input.source, cmd)
            .await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::AnalyzeGlobal {
            input: input.clone().into(),
            output: output.into(),
        },
        queued: ctx.queued,
        started: ctx.started,
        finished: Timestamp::now(),
        exit_code,
    })
}

async fn run_analyze_version(
    ctx: RunContext,
    input: analyze_version::InputV0,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&ctx.state.config.impeller.cmd);
    cmd.args(&ctx.state.config.impeller.args);
    cmd.args(&ctx.state.config.impeller.args_analyze_version);
    cmd.arg("--sha").arg(&input.sha);

    let (exit_code, output) =
        run_command_and_parse_result::<analyze_version::Output>(&ctx.state, &input.source, cmd)
            .await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::AnalyzeVersion {
            input: input.clone().into(),
            output: output.into(),
        },
        queued: ctx.queued,
        started: ctx.started,
        finished: Timestamp::now(),
        exit_code,
    })
}

async fn run_build_version(
    ctx: RunContext,
    input: build_version::InputV0,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&ctx.state.config.impeller.cmd);
    cmd.args(&ctx.state.config.impeller.args);
    cmd.args(&ctx.state.config.impeller.args_build_version);
    cmd.arg("--sha").arg(&input.sha);

    if let Some(build) = input.build {
        match build {
            true => cmd.arg("--build"),
            false => cmd.arg("--no-build"),
        };
    }

    if let Some(test) = input.test {
        match test {
            true => cmd.arg("--test"),
            false => cmd.arg("--no-test"),
        };
    }

    if let Some(lint) = input.lint {
        match lint {
            true => cmd.arg("--lint"),
            false => cmd.arg("--no-lint"),
        };
    }

    cmd.env("LEAN_NUM_THREADS", ctx.threads.to_string());

    let (exit_code, output) =
        run_command_and_parse_result::<build_version::Output>(&ctx.state, &input.source, cmd)
            .await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::BuildVersion {
            input: input.clone().into(),
            output: output.into(),
        },
        queued: ctx.queued,
        started: ctx.started,
        finished: Timestamp::now(),
        exit_code,
    })
}

struct RunContext {
    state: AppState,
    queued: Timestamp,
    started: Timestamp,
    threads: usize,
}

pub async fn run(
    state: AppState,
    input: JobInput,
    queued: Timestamp,
    started: Timestamp,
) -> anyhow::Result<JobResultV0> {
    let threads = threads_for_input(&state, &input);
    let ctx = RunContext {
        state,
        queued,
        started,
        threads,
    };
    match input {
        JobInput::AnalyzeGlobal { input } => run_analyze_global(ctx, input).await,
        JobInput::AnalyzeVersion { input } => run_analyze_version(ctx, input).await,
        JobInput::BuildVersion { input } => run_build_version(ctx, input).await,
    }
}
