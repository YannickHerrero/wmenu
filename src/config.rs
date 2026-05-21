use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Paper,
    Stone,
    Sage,
    Clay,
    Ink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeySpec(pub String);

impl Default for HotkeySpec {
    fn default() -> Self {
        Self("Alt+Space".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    pub hotkey: HotkeySpec,
    pub scan_interval_minutes: u64,
    pub extra_dirs: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            hotkey: HotkeySpec::default(),
            scan_interval_minutes: 5,
            extra_dirs: Vec::new(),
        }
    }
}

pub fn project_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "wmenu")
        .context("could not determine application data directory")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn config_path() -> Result<PathBuf> {
    Ok(project_dir()?.join("config.toml"))
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("read config: {}", path.display()))?;
        let cfg =
            toml::from_str(&text).with_context(|| format!("parse config: {}", path.display()))?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create config dir: {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serialize config")?;
        fs::write(&path, text).with_context(|| format!("write config: {}", path.display()))?;
        Ok(())
    }
}
