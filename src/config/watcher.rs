use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Config;

/// Spawns a notify::RecommendedWatcher watching the config dir. The returned
/// handle keeps the watcher alive; drop it to stop watching.
#[allow(dead_code)]
pub fn spawn(sender: Sender<Config>) -> Result<RecommendedWatcher> {
    let path: PathBuf = super::config_path()?;
    let dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .context("config path has no parent dir")?;

    let last_fire: Arc<Mutex<Instant>> =
        Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let event = match res {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("watcher error: {e}");
                return;
            }
        };

        if !event.paths.iter().any(|p| p == &path) {
            return;
        }
        if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
            return;
        }

        {
            let mut last = last_fire.lock().unwrap();
            if last.elapsed() < Duration::from_millis(250) {
                return;
            }
            *last = Instant::now();
        }

        match Config::load() {
            Ok(cfg) => {
                tracing::info!("config reloaded from disk");
                let _ = sender.send(cfg);
            }
            Err(e) => tracing::warn!("reload config failed: {e}"),
        }
    })?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}
