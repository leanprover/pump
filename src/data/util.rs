use std::fmt;

use serde::{Deserialize, Serialize};

/// A line of output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "(u8, i64, String)", from = "(u8, i64, String)")]
pub struct Line {
    /// Source of the line, see constants in [`Line`].
    pub source: u8,

    /// Timestamp of the line relative to the start of the run, in seconds.
    pub timestamp: i64,

    /// Content of the line, without trailing newline.
    pub text: String,
}

impl Line {
    /// A line emitted by pump itself.
    pub const PUMP: u8 = 0;

    /// A line written to stdout by impeller (or any of its subprocesses).
    pub const STDOUT: u8 = 1;

    /// A line written to stderr by impeller (or any of its subprocesses).
    pub const STDERR: u8 = 2;
}

impl From<Line> for (u8, i64, String) {
    fn from(line: Line) -> Self {
        (line.source, line.timestamp, line.text)
    }
}

impl From<(u8, i64, String)> for Line {
    fn from(tuple: (u8, i64, String)) -> Self {
        Self {
            source: tuple.0,
            timestamp: tuple.1,
            text: tuple.2,
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Source {
    Github { owner: String, repo: String },
}

/// Check that a name is safe to use as a single path or URL component.
///
/// Does not match GitHub's checks exactly, but should prevent path traversal
/// type attacks.
fn validate_name(what: &str, name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 100
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err(format!("invalid {what} {name:?}"))
    }
}

impl Source {
    /// Check that the source can be safely turned into paths and URLs.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Github { owner, repo } => {
                validate_name("github owner", owner)?;
                validate_name("github repo", repo)
            }
        }
    }
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Github { owner, repo } => write!(f, "github:{owner}/{repo}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github(owner: &str, repo: &str) -> Source {
        Source::Github {
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }

    #[test]
    fn valid_sources() {
        for (owner, repo) in [
            ("leanprover", "lean4"),
            ("leanprover-community", "mathlib4"),
            ("octocat_acme", ".github"),
            ("-x", "..."),
            (&"a".repeat(100), &"b".repeat(100)),
        ] {
            assert_eq!(github(owner, repo).validate(), Ok(()), "{owner}/{repo}");
        }
    }

    #[test]
    fn invalid_sources() {
        for (owner, repo) in [
            ("", "lean4"),
            (".", "lean4"),
            ("..", "lean4"),
            ("../..", "lean4"),
            ("a/b", "lean4"),
            ("a\\b", "lean4"),
            ("a\0b", "lean4"),
            ("leanprover", ""),
            ("leanprover", "."),
            ("leanprover", ".."),
            ("leanprover", "../x"),
            ("leanprover", "/etc"),
            ("a b", "lean4"),
            ("leanprover", "a?b"),
            ("leanprover", "a#b"),
            ("leanprover", "ä"),
            (&"a".repeat(101), "lean4"),
            ("leanprover", &"a".repeat(101)),
        ] {
            assert!(
                github(owner, repo).validate().is_err(),
                "{owner:?}/{repo:?}"
            );
        }
    }
}
