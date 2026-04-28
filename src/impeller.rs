use std::{fs, path::PathBuf, process::Command};

use jiff::Timestamp;
use serde::de::DeserializeOwned;
use tempfile::NamedTempFile;

use crate::{
    AppState,
    data::{
        analyze_global, analyze_version,
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

async fn run_command_and_parse_result<T: DeserializeOwned + Send + 'static>(
    state: &AppState,
    source: &SourceV0,
    mut cmd: Command,
) -> anyhow::Result<(i32, T)> {
    let output_file = NamedTempFile::new()?;

    cmd.arg("--repo").arg(repo_dir_for_source(state, source));
    cmd.arg("--url").arg(url_for_source(source));
    cmd.arg("--output").arg(output_file.path());

    // TODO Github token file

    let status = tokio::task::spawn_blocking(move || cmd.status()).await??;
    let exit_code = status.code().unwrap_or(-1);

    let json = fs::read_to_string(&output_file)?;
    let result = serde_json::from_str::<T>(&json)?;

    Ok((exit_code, result))
}

async fn run_analyze_global(
    state: &AppState,
    input: &analyze_global::InputV0,
    queued: Timestamp,
    started: Timestamp,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&state.config.impeller.cmd);
    cmd.args(&state.config.impeller.args);
    cmd.args(&state.config.impeller.args_analyze_global);

    let (exit_code, output) =
        run_command_and_parse_result::<analyze_global::Output>(state, &input.source, cmd).await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::AnalyzeGlobal {
            input: input.clone().into(),
            output: output.into(),
        },
        queued,
        started,
        finished: Timestamp::now(),
        exit_code,
    })
}

async fn run_analyze_version(
    state: &AppState,
    input: &analyze_version::InputV0,
    queued: Timestamp,
    started: Timestamp,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&state.config.impeller.cmd);
    cmd.args(&state.config.impeller.args);
    cmd.args(&state.config.impeller.args_analyze_version);
    cmd.arg("--sha").arg(&input.sha);

    let (exit_code, output) =
        run_command_and_parse_result::<analyze_version::Output>(state, &input.source, cmd).await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::AnalyzeVersion {
            input: input.clone().into(),
            output: output.into(),
        },
        queued,
        started,
        finished: Timestamp::now(),
        exit_code,
    })
}

pub async fn run(
    state: &AppState,
    input: &JobInput,
    queued: Timestamp,
    started: Timestamp,
) -> anyhow::Result<JobResultV0> {
    match input {
        JobInput::AnalyzeGlobal { input } => {
            run_analyze_global(state, input, queued, started).await
        }
        JobInput::AnalyzeVersion { input } => {
            run_analyze_version(state, input, queued, started).await
        }
    }
}
