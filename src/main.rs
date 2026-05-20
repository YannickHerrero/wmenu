#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use anyhow::Result;
use eframe::egui;

mod app;
mod config;
mod index;
mod logging;
mod matcher;
mod mru;
mod ui;

fn main() -> Result<()> {
    let _log_guard = logging::init()?;
    tracing::info!("wmenu starting");

    let cfg = config::Config::load()?;
    let shared_index = index::new_shared();
    index::spawn_scan(shared_index.clone(), cfg.extra_dirs.clone());
    let mru_store = mru::Mru::load()?;

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
        Box::new(move |_cc| Ok(Box::new(app::App::new(cfg, shared_index, mru_store)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe run: {e}"))?;
    Ok(())
}
