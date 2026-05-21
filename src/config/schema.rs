use std::path::PathBuf;

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
    #[serde(default = "default_omakase_hotkey")]
    pub omakase_hotkey: HotkeySpec,
    #[serde(default)]
    pub amphetamine_enabled: bool,
}

fn default_omakase_hotkey() -> HotkeySpec {
    HotkeySpec("Alt+Super+Space".to_string())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            hotkey: HotkeySpec::default(),
            scan_interval_minutes: 5,
            extra_dirs: Vec::new(),
            omakase_hotkey: default_omakase_hotkey(),
            amphetamine_enabled: false,
        }
    }
}
