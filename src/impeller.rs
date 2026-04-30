use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use jiff::Timestamp;
use log::{debug, error};
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

fn read_json_output<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let json = fs::read_to_string(path)?;
    let result = serde_json::from_str::<T>(&json)?;
    Ok(result)
}

async fn run_command_and_parse_result<T: DeserializeOwned + Send + 'static>(
    state: &AppState,
    source: &SourceV0,
    mut cmd: Command,
) -> anyhow::Result<(Option<T>, i32, Option<String>, Option<String>)> {
    let output_file = NamedTempFile::new()?;

    cmd.arg("--repo").arg(repo_dir_for_source(state, source));
    cmd.arg("--url").arg(url_for_source(source));
    cmd.arg("--output").arg(output_file.path());

    let cmd_str = format!("{:?}", cmd);
    debug!("Running command: {}", cmd_str);

    let output = tokio::task::spawn_blocking(move || cmd.output()).await??;
    let exit_code = output.status.code().unwrap_or(-1);
    if exit_code != 0 {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command: {cmd_str}\nExit code: {exit_code}\nStdout:\n{stdout}\nStderr:\n{stderr}");
    }

    let mut stdout = None;
    let mut stderr = None;
    if exit_code != 0 {
        stdout = Some(String::from_utf8_lossy(&output.stdout).to_string());
        stderr = Some(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let result = read_json_output(output_file.path()).ok();
    Ok((result, exit_code, stdout, stderr))
}

async fn run_analyze_global(
    ctx: RunContext,
    input: analyze_global::InputV0,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&ctx.state.config.impeller.cmd);
    cmd.arg("analyze-global");
    cmd.args(&ctx.state.config.impeller.args);
    cmd.args(&ctx.state.config.impeller.args_analyze_global);

    let (output, exit_code, stdout, stderr) =
        run_command_and_parse_result::<analyze_global::Output>(&ctx.state, &input.source, cmd)
            .await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::AnalyzeGlobal {
            input: input.clone().into(),
            output,
        },
        queued: ctx.queued,
        started: ctx.started,
        finished: Timestamp::now(),
        exit_code,
        stdout,
        stderr,
    })
}

async fn run_analyze_version(
    ctx: RunContext,
    input: analyze_version::InputV0,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&ctx.state.config.impeller.cmd);
    cmd.arg("analyze-version");
    cmd.args(&ctx.state.config.impeller.args);
    cmd.args(&ctx.state.config.impeller.args_analyze_version);
    cmd.arg("--rev").arg(&input.sha);

    let (output, exit_code, stdout, stderr) =
        run_command_and_parse_result::<analyze_version::Output>(&ctx.state, &input.source, cmd)
            .await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::AnalyzeVersion {
            input: input.clone().into(),
            output,
        },
        queued: ctx.queued,
        started: ctx.started,
        finished: Timestamp::now(),
        exit_code,
        stdout,
        stderr,
    })
}

async fn run_build_version(
    ctx: RunContext,
    input: build_version::InputV0,
) -> anyhow::Result<JobResultV0> {
    let mut cmd = Command::new(&ctx.state.config.impeller.cmd);
    cmd.arg("build-version");
    cmd.args(&ctx.state.config.impeller.args);
    cmd.args(&ctx.state.config.impeller.args_build_version);
    cmd.arg("--rev").arg(&input.sha);

    if let Some(toolchain) = &input.override_toolchain {
        cmd.arg("--override-toolchain").arg(toolchain);
    }

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

    let (output, exit_code, stdout, stderr) =
        run_command_and_parse_result::<build_version::Output>(&ctx.state, &input.source, cmd)
            .await?;

    Ok(JobResultV0 {
        data: JobResultDataV0::BuildVersion {
            input: input.clone().into(),
            output,
        },
        queued: ctx.queued,
        started: ctx.started,
        finished: Timestamp::now(),
        exit_code,
        stdout,
        stderr,
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
