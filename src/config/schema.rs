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
pub struct DaemonCfg {
    pub autostart: bool,
    pub start_minimized: bool,
}

impl Default for DaemonCfg {
    fn default() -> Self {
        Self {
            autostart: false,
            start_minimized: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherCfg {
    pub hotkey: HotkeySpec,
    pub omakase_hotkey: HotkeySpec,
    pub scan_interval_minutes: u64,
    pub extra_dirs: Vec<PathBuf>,
}

impl Default for LauncherCfg {
    fn default() -> Self {
        Self {
            hotkey: HotkeySpec::default(),
            omakase_hotkey: default_omakase_hotkey(),
            scan_interval_minutes: 5,
            extra_dirs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellKind {
    #[default]
    Powershell,
    Cmd,
    Pwsh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Launch {
        command: String,
    },
    Url {
        url: String,
    },
    Script {
        #[serde(default)]
        shell: ShellKind,
        script: String,
    },
    FocusOrLaunch {
        exe_path: String,
        #[serde(default = "default_true")]
        match_basename: bool,
        #[serde(default)]
        launch_args: Vec<String>,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub label: String,
    pub key: String,
    pub action: Action,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    pub daemon: DaemonCfg,
    pub launcher: LauncherCfg,
    pub amphetamine_enabled: bool,
    pub bindings: Vec<Binding>,
}

fn default_omakase_hotkey() -> HotkeySpec {
    HotkeySpec("Alt+Super+Space".to_string())
}
