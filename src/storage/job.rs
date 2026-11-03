use std::{fs, path::PathBuf};

use anyhow::Context;

use crate::storage::{
    data::{JobResult, JobResultV0},
    read_json, write_json,
};

/// Whether a string is a well-formed job id (a lowercase hex sha256).
pub fn is_valid_job_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Job results, stored as `{id}.json`.
pub struct JobStore {
    dir: PathBuf,
}

impl JobStore {
    pub fn new(dir: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
        Ok(Self { dir })
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// The latest result for a job id, if any. Invalid ids have no result.
    pub fn get(&self, id: &str) -> anyhow::Result<Option<JobResultV0>> {
        if !is_valid_job_id(id) {
            return Ok(None);
        }
        let result = read_json::<JobResult>(&self.path(id))?;
        Ok(result.map(JobResult::into_latest))
    }

    /// Store a result, replacing any previous result for the same input.
    pub fn put(&self, result: JobResultV0) -> anyhow::Result<()> {
        write_json(&self.path(&result.id()), &JobResult::V0(result))
    }
}
