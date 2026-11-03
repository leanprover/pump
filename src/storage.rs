mod batch;
pub mod data;
mod job;

use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Serialize, de::DeserializeOwned};
use tempfile::NamedTempFile;

pub use self::{batch::*, job::*};

/// Read and parse a JSON file, returning `None` if it doesn't exist.
fn read_json<T: DeserializeOwned>(path: &Path) -> anyhow::Result<Option<T>> {
    let json = match fs::read_to_string(path) {
        Ok(json) => json,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display()))?,
    };

    let value = serde_json::from_str(&json)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    Ok(Some(value))
}

/// Write a JSON file atomically by writing to a temporary file next to it and
/// renaming that into place.
fn write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string(value)?;

    let dir = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;

    let mut tmp = NamedTempFile::new_in(dir)
        .with_context(|| format!("failed to create temporary file in {}", dir.display()))?;

    tmp.write_all(json.as_bytes())
        .with_context(|| format!("failed to write {}", tmp.path().display()))?;

    tmp.as_file()
        .sync_all()
        .with_context(|| format!("failed to sync {}", tmp.path().display()))?;

    tmp.persist(path).map_err(|e| {
        anyhow::Error::new(e.error).context(format!(
            "failed to rename {} to {}",
            e.file.path().display(),
            path.display()
        ))
    })?;

    Ok(())
}

/// List all `*.json` files directly inside a directory.
fn list_json(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = vec![];
    for entry in fs::read_dir(dir).with_context(|| format!("failed to list {}", dir.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension() == Some("json".as_ref()) {
            paths.push(entry.path());
        }
    }
    Ok(paths)
}
