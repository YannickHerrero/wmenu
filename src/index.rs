use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use arc_swap::ArcSwap;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct Index {
    pub entries: Vec<AppEntry>,
    pub scanned_at: Option<Instant>,
}

pub type SharedIndex = Arc<ArcSwap<Index>>;

pub fn new_shared() -> SharedIndex {
    Arc::new(ArcSwap::from_pointee(Index::default()))
}

pub fn spawn_scan(shared: SharedIndex, extra_dirs: Vec<PathBuf>) {
    thread::spawn(move || {
        let entries = scan(&extra_dirs);
        shared.store(Arc::new(Index {
            entries,
            scanned_at: Some(Instant::now()),
        }));
    });
}

pub fn start_menu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let suffix = Path::new("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs");
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(appdata).join(&suffix));
    }
    if let Ok(programdata) = std::env::var("PROGRAMDATA") {
        dirs.push(PathBuf::from(programdata).join(&suffix));
    }
    dirs
}

pub fn scan(extra_dirs: &[PathBuf]) -> Vec<AppEntry> {
    let mut entries = Vec::new();
    let mut roots = start_menu_dirs();
    roots.extend(extra_dirs.iter().cloned());
    for root in &roots {
        walk(root, &mut entries);
    }
    debug!(
        "indexed {} entries from {} roots",
        entries.len(),
        roots.len()
    );
    entries
}

fn walk(dir: &Path, entries: &mut Vec<AppEntry>) {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let ftype = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ftype.is_dir() {
            walk(&path, entries);
        } else if is_lnk(&path) {
            if let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) {
                entries.push(AppEntry { name, path });
            }
        }
    }
}

fn is_lnk(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false)
}
