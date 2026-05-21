#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use anyhow::Result;
use eframe::egui;

mod amphetamine;
mod app;
mod autostart;
mod config;
mod hotkey;
mod index;
mod launch;
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

    let cfg = config::Config::load()?;
    let shared_index = index::new_shared();
    index::spawn_scan(shared_index.clone(), cfg.extra_dirs.clone());
    let mru_store = mru::Mru::load()?;
    let tray_handle = tray::build()?;
    let mut hotkey_mgr = hotkey::Manager::new()?;
    if let Err(e) = hotkey_mgr.set(&cfg.hotkey.0) {
        tracing::warn!("default hotkey {} failed: {e}", cfg.hotkey.0);
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
