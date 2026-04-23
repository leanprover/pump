use std::{
    fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::data::job::{JobQueryDataV0, JobResult};

pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    fn hash_key(key: &JobQueryDataV0) -> anyhow::Result<String> {
        Ok(Sha256::digest(serde_json::to_string(key)?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }

    fn path_for_key(&self, key: &JobQueryDataV0) -> anyhow::Result<PathBuf> {
        let hash = Self::hash_key(key)?;
        let path = self.dir.join(format!("{hash}.json"));
        Ok(path)
    }

    fn load_result(path: &Path) -> anyhow::Result<JobResult> {
        let json = fs::read_to_string(path)?;
        let result = serde_json::from_str(&json)?;
        Ok(result)
    }

    pub fn get(&self, key: &JobQueryDataV0) -> Option<JobResult> {
        let path = self.path_for_key(key).ok()?;
        let result = Self::load_result(&path).ok()?;
        Some(result)
    }

    pub fn put(&self, value: JobResult) -> anyhow::Result<()> {
        let path = self.path_for_key(&value.query_data())?;
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
            let target_path = self.path_for_key(&result.query_data())?;
            if target_path != path {
                println!("Moving {:?} to {:?}", path, target_path);
                fs::rename(path, target_path)?;
            }
        }
        Ok(())
    }
}
