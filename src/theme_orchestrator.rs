//! Ecosystem-wide theme switch.
//!
//! When the user picks a theme from the Omakase launcher, this module
//! fans the choice out to every visual surface in the user's setup:
//! wmenu itself (handled by the caller via App::apply_theme), wbar
//! (over its IPC socket), Explorer (by writing its config.json so the
//! Explorer-side notify watcher picks it up), GlazeWM (yaml border
//! colours + wm-reload-config), and the Windows registry (dark/light
//! mode + DWM accent colour, followed by a WM_SETTINGCHANGE broadcast).
//!
//! Each leg is independent — if wbar isn't running or the GlazeWM yaml
//! has been moved, the rest of the targets still get the update. The
//! per-leg `Result` is logged and returned in [`ApplyReport`] so the
//! caller can log a one-line summary.
//!
//! The wmenu leg is *not* implemented here because mutating the running
//! `App` needs `&mut self` from the UI thread; the caller handles it
//! via `App::apply_theme`. This module covers everything external.
//!
//! All actual writes are stubs in this initial scaffolding commit —
//! each leg gets wired in its own follow-up commit.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use eframe::egui::Color32;

use crate::config::Theme;
use crate::ui::theme::Palette;

/// wbar's IPC port. The protocol mirrors wmenu's own (newline-delimited
/// text, server replies `ok\n` or `error: <msg>\n`).
const WBAR_IPC_PORT: u16 = 17128;

/// Trailing-comment sentinels the user's GlazeWM config.yaml uses to mark
/// which border-color lines we should rewrite. See windot's seed config
/// for the canonical placement.
const GLAZEWM_FOCUSED_SENTINEL: &str = "# wmenu-theme-focused";
const GLAZEWM_UNFOCUSED_SENTINEL: &str = "# wmenu-theme-unfocused";

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Win32 CreateProcess flag: don't allocate a console for the child. Used
/// when spawning glazewm.exe, which is a console-subsystem binary and would
/// otherwise flash a terminal window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Per-leg outcome. `Ok(())` = applied successfully, `Err(_)` = logged
/// at warn-level and skipped, but does not abort the other legs.
pub struct ApplyReport {
    pub wbar: Result<()>,
    pub explorer: Result<()>,
    pub glazewm: Result<()>,
    pub windows: Result<()>,
    pub wezterm: Result<()>,
    pub wallpaper: Result<()>,
}

impl ApplyReport {
    /// Compact one-line summary suitable for `tracing::info!` after a
    /// theme switch. Shows each leg as `ok` or `err: <msg>`.
    pub fn summarise(&self) -> String {
        fn fmt(r: &Result<()>) -> String {
            match r {
                Ok(()) => "ok".into(),
                Err(e) => format!("err: {e}"),
            }
        }
        format!(
            "wbar={} explorer={} glazewm={} windows={} wezterm={} wallpaper={}",
            fmt(&self.wbar),
            fmt(&self.explorer),
            fmt(&self.glazewm),
            fmt(&self.windows),
            fmt(&self.wezterm),
            fmt(&self.wallpaper),
        )
    }
}

/// Fan the chosen theme out to every external target.
///
/// `palette` is passed in so the orchestrator does not need to re-derive
/// the hex values — the caller already has them.
pub fn apply(theme: Theme, palette: &Palette, terminal_monochrome: bool) -> ApplyReport {
    ApplyReport {
        wbar: apply_wbar(theme),
        explorer: apply_explorer(theme),
        glazewm: apply_glazewm(palette),
        windows: apply_windows(theme, palette),
        wezterm: apply_wezterm(theme, palette, terminal_monochrome),
        wallpaper: apply_wallpaper(theme),
    }
}

pub fn apply_terminal_colors(
    theme: Theme,
    palette: &Palette,
    terminal_monochrome: bool,
) -> Result<()> {
    apply_wezterm(theme, palette, terminal_monochrome)
}

fn apply_wbar(theme: Theme) -> Result<()> {
    // Theme's Debug repr is the canonical name (Paper / Stone / Sage /
    // Clay / Ink) — wbar's parser is case-insensitive so this is safe.
    let name = format!("{theme:?}");
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), WBAR_IPC_PORT);
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))
        .with_context(|| format!("connect {addr} (is wbar running?)"))?;
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok();
    writeln!(stream, "set-theme {name}").context("write wbar ipc command")?;

    let mut reader = BufReader::new(stream);
    let mut reply = String::new();
    reader.read_line(&mut reply).context("read wbar reply")?;
    let reply = reply.trim();
    if reply == "ok" {
        Ok(())
    } else {
        Err(anyhow!("wbar replied: {reply}"))
    }
}

fn apply_explorer(theme: Theme) -> Result<()> {
    // Explorer stores config under Tauri's app_config_dir, which on Windows
    // resolves to %APPDATA%\com.ilios.explorer. directories::BaseDirs lands
    // in the right place there and on Linux dev builds.
    let base = directories::BaseDirs::new().ok_or_else(|| anyhow!("BaseDirs unavailable"))?;
    let path = base
        .config_dir()
        .join("com.ilios.explorer")
        .join("config.json");

    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut json: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;

    // Explorer's ThemeKey is lowercase ("paper" | "stone" | …).
    let name = format!("{theme:?}").to_lowercase();
    let obj = json
        .as_object_mut()
        .ok_or_else(|| anyhow!("config root is not a JSON object"))?;
    obj.insert("theme".into(), serde_json::Value::String(name));

    let out = serde_json::to_string_pretty(&json).context("serialise updated config")?;
    std::fs::write(&path, out).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn apply_glazewm(palette: &Palette) -> Result<()> {
    let base = directories::BaseDirs::new().ok_or_else(|| anyhow!("BaseDirs unavailable"))?;
    let path = base.home_dir().join(".glzr/glazewm/config.yaml");
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;

    let accent = color_to_hex(palette.accent);
    let unfocused = color_to_hex(palette.ink_faint);

    let mut updated = String::with_capacity(content.len());
    let mut focused_found = false;
    let mut unfocused_found = false;

    for line in content.lines() {
        if line.contains(GLAZEWM_FOCUSED_SENTINEL) {
            updated.push_str(&replace_quoted_value(line, &accent));
            focused_found = true;
        } else if line.contains(GLAZEWM_UNFOCUSED_SENTINEL) {
            updated.push_str(&replace_quoted_value(line, &unfocused));
            unfocused_found = true;
        } else {
            updated.push_str(line);
        }
        updated.push('\n');
    }

    if !focused_found && !unfocused_found {
        return Err(anyhow!(
            "no `# wmenu-theme-focused` / `# wmenu-theme-unfocused` sentinels in {}",
            path.display()
        ));
    }

    std::fs::write(&path, updated).with_context(|| format!("write {}", path.display()))?;
    reload_glazewm()?;
    Ok(())
}

/// Replace the first single-quoted value in `line` with `new_value`.
/// Preserves leading indent, the `color:` key, and the trailing sentinel
/// comment. Defensive — leaves the line untouched if it has no quoted
/// value (which would mean someone moved the sentinel onto a different
/// shape of line).
fn replace_quoted_value(line: &str, new_value: &str) -> String {
    let Some(open) = line.find('\'') else {
        return line.to_string();
    };
    let Some(close_rel) = line[open + 1..].find('\'') else {
        return line.to_string();
    };
    let close = open + 1 + close_rel;
    format!("{}{}{}", &line[..=open], new_value, &line[close..])
}

fn color_to_hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

fn reload_glazewm() -> Result<()> {
    let mut cmd = Command::new("glazewm");
    cmd.args(["command", "wm-reload-config"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let status = cmd
        .status()
        .context("spawn `glazewm command wm-reload-config`")?;
    if !status.success() {
        return Err(anyhow!("glazewm reload exited with {status}"));
    }
    Ok(())
}

#[cfg(windows)]
fn apply_windows(theme: Theme, palette: &Palette) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Ink is the only dark theme; the others are light. AppsUseLightTheme
    // controls per-app chrome, SystemUsesLightTheme controls taskbar /
    // start menu — both should agree so the OS doesn't end up half-dark.
    let light_mode: u32 = if matches!(theme, Theme::Ink) { 0 } else { 1 };
    let (personalize, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .context("open Personalize key")?;
    personalize
        .set_value("AppsUseLightTheme", &light_mode)
        .context("write AppsUseLightTheme")?;
    personalize
        .set_value("SystemUsesLightTheme", &light_mode)
        .context("write SystemUsesLightTheme")?;

    // AccentColor is stored as a little-endian ABGR DWORD; alpha is
    // always 0xFF here (Windows ignores transparency on the accent).
    let abgr: u32 = 0xFF00_0000
        | ((palette.accent.b() as u32) << 16)
        | ((palette.accent.g() as u32) << 8)
        | (palette.accent.r() as u32);
    let (dwm, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\DWM")
        .context("open DWM key")?;
    dwm.set_value("AccentColor", &abgr)
        .context("write AccentColor")?;
    dwm.set_value("ColorizationColor", &abgr)
        .context("write ColorizationColor")?;

    broadcast_immersive_color_set();
    Ok(())
}

#[cfg(not(windows))]
fn apply_windows(_theme: Theme, _palette: &Palette) -> Result<()> {
    // Non-Windows dev builds: nothing to broadcast.
    Ok(())
}

/// Tell every visible top-level window that the colour theme has changed.
/// Without this broadcast, already-running apps keep painting their old
/// accent until the user logs out and back in.
#[cfg(windows)]
fn broadcast_immersive_color_set() {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    // null-terminated UTF-16; keep alive until SendMessageTimeoutW returns.
    let setting: Vec<u16> = "ImmersiveColorSet\0".encode_utf16().collect();
    let hwnd_broadcast = HWND(0xFFFF as *mut std::ffi::c_void);
    unsafe {
        SendMessageTimeoutW(
            hwnd_broadcast,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(setting.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            100,
            None,
        );
    }
}

/// Overwrite the user's `~/.wezterm-colors.lua` with a full WezTerm
/// `config.colors` table derived from the palette. The user's main
/// .wezterm.lua already loads this sidecar via
/// `wezterm.add_to_config_reload_watch_list` + `pcall(dofile, …)`, so
/// WezTerm hot-reloads as soon as we write the file — no IPC, no
/// spawn.
fn apply_wezterm(theme: Theme, palette: &Palette, terminal_monochrome: bool) -> Result<()> {
    let base = directories::BaseDirs::new().ok_or_else(|| anyhow!("BaseDirs unavailable"))?;
    let path = base.home_dir().join(".wezterm-colors.lua");
    let body = render_wezterm_colors(theme, palette, terminal_monochrome);
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn render_wezterm_colors(theme: Theme, p: &Palette, terminal_monochrome: bool) -> String {
    // Light themes treat "black" (ANSI 0) as a dark fg-ish colour and
    // "white" (ANSI 7) as a light-ish gray. Ink inverts: bg is dark so
    // ANSI 0 should also be dark; ANSI 7 = light fg.
    let dark = matches!(theme, Theme::Ink);
    let (ansi_0, ansi_7, bright_0, bright_7) = if dark {
        (p.paper, p.ink_faint, p.ink_faint, p.ink)
    } else {
        (p.ink, p.ink_faint, p.ink_soft, p.muted)
    };

    let fg = color_to_hex(p.ink);
    let bg = color_to_hex(p.paper);
    let cur = color_to_hex(p.accent);
    let cur_fg = color_to_hex(p.paper);
    let sel = color_to_hex(p.muted);

    let (ansi, brights) = if terminal_monochrome {
        monochrome_terminal_palette(theme, p, ansi_0, ansi_7, bright_0, bright_7)
    } else {
        terminal_palette(theme)
    };

    let a0 = color_to_hex(ansi[0]);
    let a1 = color_to_hex(ansi[1]);
    let a2 = color_to_hex(ansi[2]);
    let a3 = color_to_hex(ansi[3]);
    let a4 = color_to_hex(ansi[4]);
    let a5 = color_to_hex(ansi[5]);
    let a6 = color_to_hex(ansi[6]);
    let a7 = color_to_hex(ansi[7]);

    let b0 = color_to_hex(brights[0]);
    let b1 = color_to_hex(brights[1]);
    let b2 = color_to_hex(brights[2]);
    let b3 = color_to_hex(brights[3]);
    let b4 = color_to_hex(brights[4]);
    let b5 = color_to_hex(brights[5]);
    let b6 = color_to_hex(brights[6]);
    let b7 = color_to_hex(brights[7]);

    format!(
        "-- Generated by wmenu theme orchestrator. Edits will be overwritten\n\
         -- on the next Omakase theme switch.\n\
         return {{\n\
         \x20\x20foreground   = \"{fg}\",\n\
         \x20\x20background   = \"{bg}\",\n\
         \x20\x20cursor_bg    = \"{cur}\",\n\
         \x20\x20cursor_fg    = \"{cur_fg}\",\n\
         \x20\x20cursor_border = \"{cur}\",\n\
         \x20\x20selection_fg = \"{fg}\",\n\
         \x20\x20selection_bg = \"{sel}\",\n\
         \x20\x20ansi = {{\n\
         \x20\x20\x20\x20\"{a0}\", \"{a1}\", \"{a2}\", \"{a3}\",\n\
         \x20\x20\x20\x20\"{a4}\", \"{a5}\", \"{a6}\", \"{a7}\",\n\
         \x20\x20}},\n\
         \x20\x20brights = {{\n\
         \x20\x20\x20\x20\"{b0}\", \"{b1}\", \"{b2}\", \"{b3}\",\n\
         \x20\x20\x20\x20\"{b4}\", \"{b5}\", \"{b6}\", \"{b7}\",\n\
         \x20\x20}},\n\
         }}\n"
    )
}

fn terminal_palette(theme: Theme) -> ([Color32; 8], [Color32; 8]) {
    match theme {
        Theme::Paper => (
            [
                rgb(0x2B, 0x24, 0x1B),
                rgb(0xB5, 0x59, 0x3A),
                rgb(0x6E, 0x7B, 0x4F),
                rgb(0xC0, 0x8A, 0x2E),
                rgb(0x4A, 0x6B, 0x8A),
                rgb(0x8A, 0x5A, 0x6E),
                rgb(0x5A, 0x8A, 0x82),
                rgb(0x6B, 0x5E, 0x4E),
            ],
            [
                rgb(0xA4, 0x95, 0x80),
                rgb(0xCF, 0x6E, 0x4E),
                rgb(0x86, 0x97, 0x66),
                rgb(0xD8, 0xA6, 0x48),
                rgb(0x5F, 0x84, 0xA6),
                rgb(0xA5, 0x71, 0x86),
                rgb(0x72, 0xA6, 0x9D),
                rgb(0x2B, 0x24, 0x1B),
            ],
        ),
        Theme::Stone => (
            [
                rgb(0x2D, 0x33, 0x38),
                rgb(0xA8, 0x5A, 0x52),
                rgb(0x5E, 0x7B, 0x5E),
                rgb(0x9E, 0x82, 0x44),
                rgb(0x4A, 0x6B, 0x8A),
                rgb(0x76, 0x60, 0x8A),
                rgb(0x4E, 0x80, 0x88),
                rgb(0x5F, 0x69, 0x72),
            ],
            [
                rgb(0x9B, 0xA3, 0xAB),
                rgb(0xC2, 0x75, 0x6B),
                rgb(0x76, 0x96, 0x76),
                rgb(0xB8, 0x9C, 0x5C),
                rgb(0x62, 0x86, 0xA8),
                rgb(0x91, 0x7B, 0xA6),
                rgb(0x66, 0x9C, 0xA4),
                rgb(0x2D, 0x33, 0x38),
            ],
        ),
        Theme::Sage => (
            [
                rgb(0x2C, 0x35, 0x26),
                rgb(0x9E, 0x54, 0x40),
                rgb(0x3F, 0x5C, 0x32),
                rgb(0x9A, 0x80, 0x30),
                rgb(0x4F, 0x6B, 0x82),
                rgb(0x78, 0x58, 0x68),
                rgb(0x4E, 0x82, 0x7A),
                rgb(0x5E, 0x69, 0x54),
            ],
            [
                rgb(0x97, 0xA2, 0x87),
                rgb(0xBC, 0x6E, 0x58),
                rgb(0x5C, 0x7A, 0x4A),
                rgb(0xB4, 0x9A, 0x48),
                rgb(0x67, 0x84, 0x9C),
                rgb(0x93, 0x73, 0x82),
                rgb(0x66, 0x9C, 0x92),
                rgb(0x2C, 0x35, 0x26),
            ],
        ),
        Theme::Clay => (
            [
                rgb(0x3A, 0x28, 0x20),
                rgb(0x9E, 0x45, 0x21),
                rgb(0x6E, 0x7B, 0x4A),
                rgb(0xB0, 0x70, 0x28),
                rgb(0x5A, 0x6E, 0x84),
                rgb(0x8A, 0x54, 0x60),
                rgb(0x5A, 0x84, 0x78),
                rgb(0x6E, 0x55, 0x48),
            ],
            [
                rgb(0xA8, 0x90, 0x7F),
                rgb(0xC2, 0x60, 0x3A),
                rgb(0x86, 0x95, 0x62),
                rgb(0xCC, 0x8C, 0x40),
                rgb(0x71, 0x88, 0xA0),
                rgb(0xA5, 0x6F, 0x7B),
                rgb(0x73, 0xA0, 0x94),
                rgb(0x3A, 0x28, 0x20),
            ],
        ),
        Theme::Ink => (
            [
                rgb(0x1A, 0x1A, 0x1A),
                rgb(0xC9, 0x7A, 0x5A),
                rgb(0x8A, 0xA0, 0x6E),
                rgb(0xD4, 0xB3, 0x6A),
                rgb(0x7F, 0xA0, 0xC0),
                rgb(0xB5, 0x89, 0xA0),
                rgb(0x7F, 0xB0, 0xA8),
                rgb(0xC9, 0xC4, 0xB8),
            ],
            [
                rgb(0x5A, 0x57, 0x52),
                rgb(0xDD, 0x91, 0x76),
                rgb(0xA2, 0xB7, 0x88),
                rgb(0xE4, 0xC8, 0x86),
                rgb(0x9B, 0xB8, 0xD2),
                rgb(0xC9, 0xA2, 0xB6),
                rgb(0x9B, 0xC6, 0xBF),
                rgb(0xF5, 0xEF, 0xE0),
            ],
        ),
    }
}

fn monochrome_terminal_palette(
    theme: Theme,
    p: &Palette,
    ansi_0: Color32,
    ansi_7: Color32,
    bright_0: Color32,
    bright_7: Color32,
) -> ([Color32; 8], [Color32; 8]) {
    if matches!(theme, Theme::Ink) {
        return (
            [
                ansi_0,
                blend(p.accent, p.paper, 0.55),
                blend(p.accent, p.paper, 0.45),
                blend(p.accent, p.paper, 0.35),
                blend(p.accent, p.paper, 0.25),
                blend(p.accent, p.ink_faint, 0.35),
                p.ink_soft,
                ansi_7,
            ],
            [
                bright_0,
                blend(p.accent, p.paper, 0.25),
                blend(p.accent, p.paper, 0.15),
                p.accent,
                blend(p.accent, p.ink, 0.10),
                blend(p.accent, p.ink_faint, 0.20),
                p.ink_soft,
                bright_7,
            ],
        );
    }

    (
        [
            ansi_0,
            blend(p.accent, p.ink, 0.30),
            blend(p.accent, p.ink_soft, 0.35),
            blend(p.accent, p.paper, 0.35),
            p.accent,
            blend(p.accent, p.ink, 0.12),
            blend(p.accent, p.muted, 0.45),
            ansi_7,
        ],
        [
            bright_0,
            blend(p.accent, p.ink, 0.18),
            blend(p.accent, p.ink_soft, 0.20),
            blend(p.accent, p.paper, 0.20),
            blend(p.accent, p.ink, 0.06),
            p.accent,
            blend(p.accent, p.muted, 0.25),
            bright_7,
        ],
    )
}

fn blend(a: Color32, b: Color32, b_weight: f32) -> Color32 {
    let w = b_weight.clamp(0.0, 1.0);
    let inv = 1.0 - w;
    Color32::from_rgb(
        ((a.r() as f32 * inv) + (b.r() as f32 * w)).round() as u8,
        ((a.g() as f32 * inv) + (b.g() as f32 * w)).round() as u8,
        ((a.b() as f32 * inv) + (b.b() as f32 * w)).round() as u8,
    )
}

/// Theme-switch wallpaper leg: pick a fresh random image from the new
/// theme's pool. There's no prior pick to avoid here, so any entry is fair.
fn apply_wallpaper(theme: Theme) -> Result<()> {
    pick_wallpaper(theme, None).map(|_| ())
}

/// Pick a random `<theme>-*.png` from `%APPDATA%\wmenu\wallpapers\`, set it
/// as the desktop wallpaper, and return the chosen path. When `avoid` is
/// supplied and the pool holds more than one image, the result is
/// guaranteed to differ from it — so the rotation timer never shows the
/// same wallpaper twice in a row. An empty pool or a Win32 failure surfaces
/// as an error; the other theme legs still apply.
pub fn pick_wallpaper(theme: Theme, avoid: Option<&Path>) -> Result<PathBuf> {
    let dir = wallpapers_dir()?;
    let mut pool = theme_pool(&dir, theme)?;
    if pool.is_empty() {
        return Err(anyhow!("no wallpaper for {theme:?} in {}", dir.display()));
    }
    pool.sort();
    let avoid_idx = avoid.and_then(|a| pool.iter().position(|p| p == a));
    let chosen = pool.swap_remove(random_index(pool.len(), avoid_idx));
    set_desktop_wallpaper(&chosen)?;
    Ok(chosen)
}

fn wallpapers_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().ok_or_else(|| anyhow!("BaseDirs unavailable"))?;
    Ok(base.config_dir().join("wmenu").join("wallpapers"))
}

/// Every `<theme>-*.png` in `dir`. The trailing `-` in the prefix keeps the
/// match anchored to the rotation naming and rules out a stray `<theme>.png`
/// orphan (left behind by older syncs) leaking into the pool.
fn theme_pool(dir: &Path, theme: Theme) -> Result<Vec<PathBuf>> {
    let prefix = format!("{}-", format!("{theme:?}").to_lowercase());
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        // A missing dir is just an empty pool; the caller turns that into the
        // "no wallpaper for <theme>" error with the full path attached.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("read {}", dir.display())),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_png = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"));
        let named = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.to_ascii_lowercase().starts_with(&prefix));
        if is_png && named {
            out.push(path);
        }
    }
    Ok(out)
}

/// A process-reseeded pseudo-random index in `0..len`. When `avoid` is
/// `Some(i)` and `len > 1`, the result is drawn from `0..len` excluding that
/// index. `RandomState` reseeds from OS entropy on construction, giving
/// fresh randomness without a `rand` dependency — selection quality is
/// irrelevant here, only that consecutive picks can differ.
fn random_index(len: usize, avoid: Option<usize>) -> usize {
    use std::hash::{BuildHasher, Hasher};
    let r = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish() as usize;
    match avoid {
        Some(a) if len > 1 && a < len => {
            let pick = r % (len - 1);
            if pick >= a { pick + 1 } else { pick }
        }
        _ => r % len,
    }
}

#[cfg(windows)]
fn set_desktop_wallpaper(path: &std::path::Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_SETDESKWALLPAPER, SPIF_SENDWININICHANGE, SPIF_UPDATEINIFILE, SystemParametersInfoW,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(wide.as_ptr() as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDWININICHANGE,
        )
    }
    .ok()
    .context("SystemParametersInfoW SPI_SETDESKWALLPAPER")
}

#[cfg(not(windows))]
fn set_desktop_wallpaper(_path: &std::path::Path) -> Result<()> {
    Ok(())
}
