#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use anyhow::Result;
use eframe::egui;

mod action;
mod amphetamine;
mod app;
mod autostart;
mod config;
mod hotkey;
mod hotkey_spec;
mod index;
// Server-side wiring lands in the next commits; the module is here now so
// the CLI client (also in the next commit) can use ipc::send.
mod ipc;
mod launch;
mod lnk;
mod logging;
mod matcher;
mod mru;
mod omakase;
mod single_instance;
mod tray;
mod ui;

fn main() -> Result<()> {
    let _log_guard = logging::init()?;
    tracing::info!("wmenu starting");

    if let Err(e) = single_instance::ensure() {
        tracing::warn!("{e}");
        return Ok(());
    }

    let mut cfg = config::Config::load()?;
    // Reconcile the in-config autostart flag with the actual registry state so
    // the settings UI reflects reality on startup.
    cfg.daemon.autostart = autostart::is_enabled().unwrap_or(cfg.daemon.autostart);
    let shared_index = index::new_shared();
    index::spawn_scan(shared_index.clone(), cfg.launcher.extra_dirs.clone());
    let mru_store = mru::Mru::load()?;
    let tray_handle = tray::build()?;
    let mut hotkey_mgr = hotkey::Manager::new()?;
    if let Err(e) = hotkey_mgr.set(&cfg.launcher.hotkey.0) {
        tracing::warn!("default hotkey {} failed: {e}", cfg.launcher.hotkey.0);
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title("wmenu")
        .with_visible(false)
        .with_decorations(false)
        .with_always_on_top()
        .with_resizable(false)
        .with_inner_size([640.0, 400.0]);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "wmenu",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::App::new(
                cfg,
                shared_index,
                mru_store,
                tray_handle,
                hotkey_mgr,
                cc.egui_ctx.clone(),
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe run: {e}"))?;
    Ok(())
}
