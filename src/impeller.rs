use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
};

use jiff::Timestamp;
use log::{debug, warn};
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
};

use crate::{
    AppState,
    data::{JobInput, Line, OutputVersion, Source},
    storage::data::JobResultV0,
};

/// The log of a run, shared so it can be read while the run is in progress.
pub type Log = Arc<Mutex<Vec<Line>>>;

fn repo_dir_for_source(state: &AppState, source: &Source) -> PathBuf {
    match source {
        Source::Github { owner, repo } => state.repos_dir.join("github").join(owner).join(repo),
    }
}

fn url_for_source(source: &Source) -> String {
    match source {
        Source::Github { owner, repo } => {
            format!("https://github.com/{owner}/{repo}.git")
        }
    }
}

pub fn threads_for_input(state: &AppState, input: &JobInput) -> usize {
    match &input {
        JobInput::AnalyzeGlobal(_) => 1,
        JobInput::AnalyzeVersion(_) => 1,
        JobInput::BuildVersion(_) => state.config.queue.threads_build_version,
    }
}

/// The output version a run of the current impeller would produce.
pub fn version_for_input(state: &AppState, input: &JobInput) -> OutputVersion {
    let impeller = &state.config.impeller;
    match input {
        JobInput::AnalyzeGlobal(_) => impeller.version_analyze_global,
        JobInput::AnalyzeVersion(_) => impeller.version_analyze_version,
        JobInput::BuildVersion(_) => impeller.version_build_version,
    }
}

fn command_for_input(state: &AppState, input: &JobInput, threads: usize) -> Command {
    let impeller = &state.config.impeller;
    let mut cmd = Command::new(&impeller.cmd);
    cmd.env("LEAN_NUM_THREADS", threads.to_string());

    match input {
        JobInput::AnalyzeGlobal(_) => {
            cmd.arg("analyze-global");
            cmd.args(&impeller.args);
            cmd.args(&impeller.args_analyze_global);
        }

        JobInput::AnalyzeVersion(input) => {
            cmd.arg("analyze-version");
            cmd.args(&impeller.args);
            cmd.args(&impeller.args_analyze_version);
            cmd.arg("--rev").arg(&input.sha);
        }

        JobInput::BuildVersion(input) => {
            cmd.arg("build-version");
            cmd.args(&impeller.args);
            cmd.args(&impeller.args_build_version);
            cmd.arg("--rev").arg(&input.sha);

            if let Some(toolchain) = &input.override_toolchain {
                cmd.arg("--override-toolchain").arg(toolchain);
            }

            for (flag, value) in [
                ("build", input.build),
                ("test", input.test),
                ("lint", input.lint),
            ] {
                match value {
                    Some(true) => cmd.arg(format!("--{flag}")),
                    Some(false) => cmd.arg(format!("--no-{flag}")),
                    None => &mut cmd,
                };
            }
        }
    }

    let source = input.source();
    cmd.arg("--repo").arg(repo_dir_for_source(state, source));
    cmd.arg("--url").arg(url_for_source(source));
    cmd
}

fn push_line(log: &Log, started: Timestamp, source: u8, text: String) {
    let timestamp = Timestamp::now().duration_since(started).as_secs();
    log.lock().unwrap().push(Line {
        source,
        timestamp,
        text,
    });
}

async fn read_lines(reader: impl AsyncRead + Unpin, source: u8, log: &Log, started: Timestamp) {
    let mut reader = BufReader::new(reader);
    let mut buf = vec![];
    loop {
        buf.clear();

        // reader.read_line would error when non-UTF-8 bytes are encountered.
        // This method should be a bit more robust than that.
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let line = buf.strip_suffix(b"\n").unwrap_or(&buf);
                let text = String::from_utf8_lossy(line).into_owned();
                push_line(log, started, source, text);
            }
            Err(e) => {
                push_line(
                    log,
                    started,
                    Line::PUMP,
                    format!("Failed to read output: {e}"),
                );
                break;
            }
        }
    }
}

/// Read impeller's output file. Impeller only writes it on success, so an
/// empty file means there's no output.
fn read_output(path: &Path) -> anyhow::Result<Option<Value>> {
    let json = fs::read_to_string(path)?;
    if json.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&json)?))
}

async fn run_command(
    mut cmd: Command,
    log: &Log,
    started: Timestamp,
) -> anyhow::Result<(Option<Value>, i32)> {
    let output_file = NamedTempFile::new()?;

    cmd.arg("--output").arg(output_file.path());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    debug!("Running command: {cmd:?}");
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().expect("stdout should be piped");
    let stderr = child.stderr.take().expect("stderr should be piped");

    let (_, _, status) = tokio::join!(
        read_lines(stdout, Line::STDOUT, log, started),
        read_lines(stderr, Line::STDERR, log, started),
        child.wait(),
    );
    let exit_code = status?.code().unwrap_or(-1);

    let output = read_output(output_file.path()).unwrap_or_else(|e| {
        push_line(
            log,
            started,
            Line::PUMP,
            format!("Failed to read output file: {e}"),
        );
        None
    });

    Ok((output, exit_code))
}

/// Run impeller for an input, streaming its output into `log`. Always produces
/// a result. Errors are recorded in the log.
pub async fn run(
    state: &AppState,
    input: JobInput,
    queued: Timestamp,
    started: Timestamp,
    threads: usize,
    log: Log,
) -> JobResultV0 {
    let cmd = command_for_input(state, &input, threads);

    let (output, exit_code) = run_command(cmd, &log, started).await.unwrap_or_else(|e| {
        warn!("Failed to run impeller for {input:?}: {e:#}");
        push_line(
            &log,
            started,
            Line::PUMP,
            format!("Failed to run impeller: {e:#}"),
        );
        (None, -1)
    });

    let log = log.lock().unwrap().clone();

    JobResultV0 {
        version: version_for_input(state, &input),
        app_version: state.config.impeller.version,
        input,
        output,
        queued,
        started,
        finished: Timestamp::now(),
        exit_code,
        log: Some(log),
    }
}
