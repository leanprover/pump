use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Mutex,
};

use anyhow::bail;
use jiff::Timestamp;
use log::{debug, warn};
use serde_json::Value;
use tempfile::NamedTempFile;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    process::Command,
};

use crate::{
    AppState,
    data::{JobInput, Line, OutputVersion, Source},
    storage::data::JobResultV0,
};

/// Approximate per-line overhead of a stored log line beyond its text, in
/// bytes. A line is serialized like `[1,12345,""],`.
const LINE_OVERHEAD: usize = 16;

/// The log of a run, shared so it can be read while the run is in progress.
pub struct Log {
    pub lines: Vec<Line>,
    /// Approximate size of the stored log so far, in bytes (text plus
    /// [`LINE_OVERHEAD`] per line).
    bytes: usize,
    /// Once `bytes` would exceed this, further output is skipped.
    max_bytes: usize,
    /// Whether output is being skipped.
    full: bool,
}

impl Log {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            lines: vec![],
            bytes: 0,
            max_bytes,
            full: false,
        }
    }
}

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

/// Append a line to the log. Lines by pump itself are always appended. Other
/// lines are skipped once the log has reached its size limit.
///
/// Returns false if the line was skipped.
fn push_line(log: &Mutex<Log>, started: Timestamp, source: u8, text: String) -> bool {
    let timestamp = Timestamp::now().duration_since(started).as_secs();
    let mut log = log.lock().unwrap();

    if source != Line::PUMP {
        if log.full {
            return false;
        }
        let bytes = log.bytes.saturating_add(text.len() + LINE_OVERHEAD);
        if bytes > log.max_bytes {
            log.full = true;
            log.lines.push(Line {
                source: Line::PUMP,
                timestamp,
                text: "Log too large, skipping remaining output".to_string(),
            });
            return false;
        }
        log.bytes = bytes;
    }

    log.lines.push(Line {
        source,
        timestamp,
        text,
    });
    true
}

async fn read_lines(
    reader: impl AsyncRead + Unpin,
    source: u8,
    log: &Mutex<Log>,
    started: Timestamp,
) {
    let max_bytes = log.lock().unwrap().max_bytes;
    let mut reader = BufReader::new(reader);
    let mut buf = vec![];
    loop {
        buf.clear();

        // reader.read_line would error when non-UTF-8 bytes are encountered.
        // This method should be a bit more robust than that.
        //
        // A line longer than the whole log limit can never fit, so reading is
        // capped there to avoid buffering arbitrarily long lines.
        let mut limited = (&mut reader).take(max_bytes as u64 + 1);
        match limited.read_until(b'\n', &mut buf).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let line = buf.strip_suffix(b"\n").unwrap_or(&buf);
                let text = String::from_utf8_lossy(line).into_owned();
                if !push_line(log, started, source, text) {
                    // Keep draining the pipe so the process doesn't block.
                    if let Err(e) = tokio::io::copy(&mut reader, &mut tokio::io::sink()).await {
                        push_line(
                            log,
                            started,
                            Line::PUMP,
                            format!("Failed to read output: {e}"),
                        );
                    }
                    break;
                }
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

/// Read impeller's output file. Impeller only writes it on success, so an empty
/// file means there's no output. Files larger than `max_bytes` are rejected.
fn read_output(path: &Path, max_bytes: u64) -> anyhow::Result<Option<Value>> {
    let size = fs::metadata(path)?.len();
    if size > max_bytes {
        bail!("output file is {size} bytes, exceeding the limit of {max_bytes} bytes");
    }
    let json = fs::read_to_string(path)?;
    if json.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&json)?))
}

async fn run_command(
    mut cmd: Command,
    log: &Mutex<Log>,
    started: Timestamp,
    max_output_bytes: u64,
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

    let output = read_output(output_file.path(), max_output_bytes).unwrap_or_else(|e| {
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
    log: &Mutex<Log>,
) -> JobResultV0 {
    let cmd = command_for_input(state, &input, threads);

    let max_output_bytes = state.config.impeller.max_output_bytes;
    let result = run_command(cmd, log, started, max_output_bytes).await;
    let (output, exit_code) = result.unwrap_or_else(|e| {
        warn!("Failed to run impeller for {input:?}: {e:#}");
        push_line(
            log,
            started,
            Line::PUMP,
            format!("Failed to run impeller: {e:#}"),
        );
        (None, -1)
    });

    let log = log.lock().unwrap().lines.clone();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn log_is_truncated() {
        // Fits "abc" and "defg", but not "hij".
        let log = Mutex::new(Log::new(2 * LINE_OVERHEAD + 7));
        let started = Timestamp::now();
        let input: &[u8] = b"abc\ndefg\nhij\nk\n";
        read_lines(input, Line::STDOUT, &log, started).await;
        push_line(&log, started, Line::PUMP, "error".to_string());

        let log = log.lock().unwrap();
        let texts = log
            .lines
            .iter()
            .map(|l| (l.source, l.text.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                (Line::STDOUT, "abc"),
                (Line::STDOUT, "defg"),
                (Line::PUMP, "Log too large, skipping remaining output"),
                (Line::PUMP, "error"),
            ]
        );
    }

    #[tokio::test]
    async fn long_line_is_not_buffered() {
        let log = Mutex::new(Log::new(100));
        let input = vec![b'x'; 1000];
        read_lines(&input[..], Line::STDERR, &log, Timestamp::now()).await;
        let log = log.lock().unwrap();
        assert_eq!(log.lines.len(), 1);
        assert_eq!(log.lines[0].source, Line::PUMP);
    }
}
