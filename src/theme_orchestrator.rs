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
use std::time::Duration;

use anyhow::{Context, Result, anyhow};

use crate::config::Theme;
use crate::ui::theme::Palette;

/// wbar's IPC port. The protocol mirrors wmenu's own (newline-delimited
/// text, server replies `ok\n` or `error: <msg>\n`).
const WBAR_IPC_PORT: u16 = 17128;

/// Per-leg outcome. `Ok(())` = applied successfully, `Err(_)` = logged
/// at warn-level and skipped, but does not abort the other legs.
pub struct ApplyReport {
    pub wbar: Result<()>,
    pub explorer: Result<()>,
    pub glazewm: Result<()>,
    pub windows: Result<()>,
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
            "wbar={} explorer={} glazewm={} windows={}",
            fmt(&self.wbar),
            fmt(&self.explorer),
            fmt(&self.glazewm),
            fmt(&self.windows),
        )
    }
}

/// Fan the chosen theme out to every external target.
///
/// `palette` is passed in so the orchestrator does not need to re-derive
/// the hex values — the caller already has them.
pub fn apply(theme: Theme, palette: &Palette) -> ApplyReport {
    ApplyReport {
        wbar: apply_wbar(theme),
        explorer: apply_explorer(theme),
        glazewm: apply_glazewm(palette),
        windows: apply_windows(theme, palette),
    }
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

fn apply_explorer(_theme: Theme) -> Result<()> {
    // TODO: edit %APPDATA%\com.ilios.explorer\config.json — only the
    // `theme` field. Explorer's notify watcher (v1.1.0+) picks it up.
    Ok(())
}

fn apply_glazewm(_palette: &Palette) -> Result<()> {
    // TODO: yaml-edit ~/.glzr/glazewm/config.yaml border colours, then
    // spawn `glazewm command wm-reload-config`.
    Ok(())
}

fn apply_windows(_theme: Theme, _palette: &Palette) -> Result<()> {
    // TODO: HKCU Personalize light/dark + DWM AccentColor / ColorizationColor,
    // then SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE,
    // "ImmersiveColorSet").
    Ok(())
}
