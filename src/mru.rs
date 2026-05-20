use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::config;

const MAX_ENTRIES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MruEntry {
    pub path: PathBuf,
    pub last_used: u64,
    pub count: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Mru {
    pub entries: Vec<MruEntry>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn mru_path() -> Result<PathBuf> {
    Ok(config::project_dir()?.join("mru.json"))
}

impl Mru {
    pub fn load() -> Result<Self> {
        let path = mru_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            fs::read_to_string(&path).with_context(|| format!("read mru: {}", path.display()))?;
        match serde_json::from_str(&text) {
            Ok(mru) => Ok(mru),
            Err(e) => {
                warn!("corrupt mru.json ({}), starting fresh", e);
                Ok(Self::default())
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = mru_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create mru dir: {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).context("serialize mru")?;
        fs::write(&path, text).with_context(|| format!("write mru: {}", path.display()))?;
        Ok(())
    }

    pub fn record_launch(&mut self, path: &Path) {
        let now = now_secs();
        if let Some(existing) = self.entries.iter_mut().find(|e| e.path == path) {
            existing.last_used = now;
            existing.count = existing.count.saturating_add(1);
        } else {
            self.entries.push(MruEntry {
                path: path.to_path_buf(),
                last_used: now,
                count: 1,
            });
        }
        self.entries.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
    }

    pub fn boost(&self, path: &Path) -> f32 {
        match self.entries.iter().position(|e| e.path == path) {
            Some(i) => (MAX_ENTRIES - i) as f32 / MAX_ENTRIES as f32,
            None => 0.0,
        }
    }
}
