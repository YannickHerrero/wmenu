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
mod theme_orchestrator;
mod tray;
mod ui;

fn main() -> Result<()> {
    // CLI client mode: any first argv that matches a known subcommand sends
    // the command to the already-running wmenu over IPC and exits. Runs
    // *before* single_instance::ensure() so the client doesn't trip the
    // singleton mutex held by the running daemon.
    if let Some(code) = handle_cli() {
        std::process::exit(code);
    }

    let _log_guard = logging::init()?;
    tracing::info!("wmenu starting");

    // Release builds use `windows_subsystem = "windows"`, so the process has
    // no stderr. Without these hooks, a panic or any error propagated through
    // `?` vanishes — the user sees `wmenu starting` in the log file and then
    // nothing, with no clue what failed. Route both into the log instead.
    std::panic::set_hook(Box::new(|info| tracing::error!("panic: {info}")));
    let outcome = run();
    if let Err(err) = &outcome {
        tracing::error!("fatal startup error: {err:#}");
    }
    outcome
}

fn run() -> Result<()> {
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
            let ipc_rx = match ipc::spawn(cc.egui_ctx.clone()) {
                Ok(rx) => Some(rx),
                Err(err) => {
                    tracing::warn!(error = ?err, "ipc control server disabled");
                    None
                }
            };
            Ok(Box::new(app::App::new(
                cfg,
                shared_index,
                mru_store,
                tray_handle,
                hotkey_mgr,
                cc.egui_ctx.clone(),
                ipc_rx,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe run: {e}"))?;
    Ok(())
}

/// Inspect argv. Returns `Some(exit_code)` if a subcommand was handled
/// (the process should exit with that code); `None` if no subcommand was
/// given and the daemon should run normally.
fn handle_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return None;
    }
    let cmd = match args[1].as_str() {
        "--help" | "-h" | "help" => {
            print_usage(&args[0]);
            return Some(0);
        }
        "set-theme" => {
            let Some(name) = args.get(2) else {
                eprintln!("set-theme requires a theme name");
                eprintln!("usage: {} set-theme <Paper|Stone|Sage|Clay|Ink>", args[0]);
                return Some(2);
            };
            format!("set-theme {name}")
        }
        other => {
            eprintln!("unknown command: {other}");
            print_usage(&args[0]);
            return Some(2);
        }
    };

    match ipc::send(&cmd) {
        Ok(reply) => {
            if let Some(rest) = reply.strip_prefix("error:") {
                eprintln!("error:{rest}");
                Some(1)
            } else {
                Some(0)
            }
        }
        Err(err) => {
            eprintln!("ipc error: {err:#}");
            Some(1)
        }
    }
}

fn print_usage(prog: &str) {
    eprintln!("wmenu — keyboard-driven Windows utility");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  {prog}                     Run the daemon (no arguments)");
    eprintln!("  {prog} set-theme <name>    Switch theme (Paper|Stone|Sage|Clay|Ink)");
    eprintln!("  {prog} --help              Show this message");
}
