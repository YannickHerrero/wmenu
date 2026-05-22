use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Config;

/// Most-recently-written config text. Shared between the app's save path and
/// the watcher: when the file we just wrote bubbles back through `notify`,
/// the watcher compares the new on-disk text to this value and skips the
/// reload if they match, so auto-saving doesn't trigger a feedback loop.
pub type LastWritten = Arc<Mutex<Option<String>>>;

pub fn make_last_written() -> LastWritten {
    Arc::new(Mutex::new(None))
}

/// Spawns a notify::RecommendedWatcher watching the config dir. The returned
/// handle keeps the watcher alive; drop it to stop watching.
#[allow(dead_code)]
pub fn spawn(sender: Sender<Config>, last_written: LastWritten) -> Result<RecommendedWatcher> {
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

        // Self-write guard: if the file on disk matches what we just wrote
        // ourselves, the auto-save fired and there's nothing new to learn.
        if let Ok(on_disk) = fs::read_to_string(&path)
            && let Ok(guard) = last_written.lock()
            && guard.as_ref() == Some(&on_disk)
        {
            return;
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
