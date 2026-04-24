use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::data::{
    analyze_global, analyze_version,
    job::{JobQuery, JobQueryDataV0, JobQueryV0, JobResult, JobResultDataV0, JobResultV0},
};

#[derive(Clone, PartialEq, Eq, Serialize)]
pub enum JobId {
    AnalyzeGlobal { input: analyze_global::InputV0 },
    AnalyzeVersion { input: analyze_version::InputV0 },
}

impl JobQueryDataV0 {
    pub fn id(&self) -> JobId {
        match self {
            JobQueryDataV0::AnalyzeGlobal { input } => JobId::AnalyzeGlobal {
                input: input.clone().into(),
            },
            JobQueryDataV0::AnalyzeVersion { input } => JobId::AnalyzeVersion {
                input: input.clone().into(),
            },
        }
    }
}

impl JobQueryV0 {
    pub fn id(&self) -> JobId {
        self.data.id()
    }
}

impl JobQuery {
    pub fn id(&self) -> JobId {
        match self {
            JobQuery::V0(job) => job.id(),
        }
    }
}

impl JobResultDataV0 {
    pub fn id(&self) -> JobId {
        match self {
            JobResultDataV0::AnalyzeGlobal { input, .. } => JobId::AnalyzeGlobal {
                input: input.clone().into(),
            },
            JobResultDataV0::AnalyzeVersion { input, .. } => JobId::AnalyzeVersion {
                input: input.clone().into(),
            },
        }
    }
}

impl JobResultV0 {
    pub fn id(&self) -> JobId {
        self.data.id()
    }
}

impl JobResult {
    pub fn id(&self) -> JobId {
        match self {
            JobResult::V0(result) => result.id(),
        }
    }
}

pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn hash_key(key: &JobId) -> anyhow::Result<String> {
        Ok(Sha256::digest(serde_json::to_string(key)?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }

    fn path_for_key(&self, key: &JobId) -> anyhow::Result<PathBuf> {
        let hash = Self::hash_key(key)?;
        let path = self.dir.join(format!("{hash}.json"));
        Ok(path)
    }

    fn load_result(path: &Path) -> anyhow::Result<JobResult> {
        let json = fs::read_to_string(path)?;
        let result = serde_json::from_str(&json)?;
        Ok(result)
    }

    pub fn get(&self, key: &JobId) -> Option<JobResult> {
        let path = self.path_for_key(key).ok()?;
        let result = Self::load_result(&path).ok()?;
        Some(result)
    }

    pub fn put(&self, value: JobResult) -> anyhow::Result<()> {
        let path = self.path_for_key(&value.id())?;
        let json = serde_json::to_string(&value)?;

        let mut tmp_path = path.clone();
        tmp_path.add_extension("tmp");

        fs::create_dir_all(&self.dir)?;
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    fn find_entries(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut entries: Vec<PathBuf> = vec![];
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            if !entry.file_name().to_string_lossy().ends_with(".json") {
                continue;
            }
            entries.push(entry.path());
        }
        Ok(entries)
    }

    /// Recompute the hashes for all entries in the cache, renaming them if necessary.
    pub fn fix_entries(&self) -> anyhow::Result<()> {
        for path in self.find_entries()? {
            let result = Self::load_result(&path)?;
            let target_path = self.path_for_key(&result.id())?;
            if target_path != path {
                println!("Moving {:?} to {:?}", path, target_path);
                fs::rename(path, target_path)?;
            }
        }
        Ok(())
    }
}
