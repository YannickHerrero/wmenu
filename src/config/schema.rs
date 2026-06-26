use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Paper,
    Stone,
    Sage,
    Clay,
    Ink,
}

impl FromStr for Theme {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "paper" => Ok(Self::Paper),
            "stone" => Ok(Self::Stone),
            "sage" => Ok(Self::Sage),
            "clay" => Ok(Self::Clay),
            "ink" => Ok(Self::Ink),
            _ => Err(format!(
                "unknown theme {s:?} (expected Paper, Stone, Sage, Clay, or Ink)"
            )),
        }
    }
}

// A stray capital in `config.toml` (e.g. `theme = "Stone"`) used to fail
// serde's lowercase-only matching, propagate `Err` up through `main()`, and
// silently kill the daemon because release builds have no stderr
// (`windows_subsystem = "windows"`). Delegate to the case-insensitive
// `FromStr` impl so on-disk hand-edits with capitalised variant names load.
impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
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
    /// Re-randomize the desktop wallpaper within the active theme's pool.
    /// Stateful (needs the running app's theme + last pick), so it's
    /// dispatched in `App`, not the stateless `action::run`.
    RotateWallpaper,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    pub daemon: DaemonCfg,
    pub launcher: LauncherCfg,
    pub amphetamine_enabled: bool,
    /// Hide the OS titlebar / window chrome on the settings window. Esc still
    /// closes it; without a titlebar there's no draggable region (rely on the
    /// window manager, e.g. Win+drag on Windows).
    pub settings_borderless: bool,
    /// When true, generated terminal ANSI palettes stay within the active
    /// theme's hue instead of using semantic red / green / yellow slots.
    pub terminal_monochrome: bool,
    /// Minutes between desktop-wallpaper re-randomizations. Each tick picks a
    /// fresh `<theme>-*.png` from the active theme's pool. `0` disables
    /// rotation (the wallpaper still changes once per theme switch).
    pub wallpaper_rotation_minutes: u64,
    pub bindings: Vec<Binding>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            daemon: DaemonCfg::default(),
            launcher: LauncherCfg::default(),
            amphetamine_enabled: false,
            settings_borderless: false,
            terminal_monochrome: true,
            wallpaper_rotation_minutes: 30,
            bindings: Vec::new(),
        }
    }
}

fn default_omakase_hotkey() -> HotkeySpec {
    HotkeySpec("Alt+Super+Space".to_string())
}
