use std::{fs, io::ErrorKind, path::PathBuf};

use anyhow::{Context, bail};

use crate::storage::{
    data::{Batch, BatchV0},
    list_json, read_json, write_json,
};

/// Whether a string is a well-formed batch id.
pub fn is_valid_batch_id(id: &str) -> bool {
    super::is_valid_job_id(id)
}

/// Where a batch is in its lifecycle. Batches only ever move forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatchState {
    /// The batch is active and affects the queue.
    Running,
    /// The batch is no longer active, but has not finished its cleanup (e.g.
    /// because the callback URL hasn't been successfully contacted yet).
    Stopping,
    /// The batch is no longer active.
    Stopped,
}

impl BatchState {
    /// In lifecycle order.
    pub const ALL: [Self; 3] = [Self::Running, Self::Stopping, Self::Stopped];

    fn dir_name(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
        }
    }
}

/// Batches, stored as `{state}/{id}.json`.
///
/// To prevent issues from concurrent modifications, some functions take a `&mut
/// BatchStore` even though technically the `mut` could be omitted.
pub struct BatchStore {
    dir: PathBuf,
}

impl BatchStore {
    pub fn new(dir: PathBuf) -> anyhow::Result<Self> {
        for state in BatchState::ALL {
            let dir = dir.join(state.dir_name());
            fs::create_dir_all(&dir)
                .with_context(|| format!("failed to create {}", dir.display()))?;
        }
        Ok(Self { dir })
    }

    fn path(&self, state: BatchState, id: &str) -> PathBuf {
        self.dir.join(state.dir_name()).join(format!("{id}.json"))
    }

    /// Find a batch by id. Invalid ids have no batch.
    pub fn get(&self, id: &str) -> anyhow::Result<Option<(BatchState, BatchV0)>> {
        if !is_valid_batch_id(id) {
            return Ok(None);
        }
        for state in BatchState::ALL {
            if let Some(batch) = read_json::<Batch>(&self.path(state, id))? {
                return Ok(Some((state, batch.into_latest())));
            }
        }
        Ok(None)
    }

    /// List all batches in a specific state.
    pub fn list(&self, state: BatchState) -> anyhow::Result<Vec<BatchV0>> {
        let mut batches = vec![];
        for path in list_json(&self.dir.join(state.dir_name()))? {
            if let Some(batch) = read_json::<Batch>(&path)? {
                batches.push(batch.into_latest());
            }
        }
        Ok(batches)
    }

    /// Store a new batch, failing if a batch with the same id already exists.
    pub fn create(&mut self, state: BatchState, batch: BatchV0) -> anyhow::Result<()> {
        let id = &batch.id;
        if !is_valid_batch_id(id) {
            bail!("invalid batch id {id:?}");
        }
        if self.get(id)?.is_some() {
            bail!("batch {id} already exists");
        }
        write_json(&self.path(state, id), &Batch::V0(batch))
    }

    /// Move a batch from one state to a later one.
    pub fn advance(&mut self, id: &str, from: BatchState, to: BatchState) -> anyhow::Result<()> {
        if !is_valid_batch_id(id) {
            bail!("invalid batch id {id:?}");
        }
        if from >= to {
            bail!("batches can't move from {from:?} to {to:?}");
        }
        match fs::rename(self.path(from, id), self.path(to, id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                bail!("batch {id} not found in state {from:?}")
            }
            Err(e) => Err(e).with_context(|| format!("failed to move batch {id} to {to:?}")),
        }
    }
}
