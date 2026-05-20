use std::fs;
use std::path::{Path, PathBuf};

use lnk::ShellLink;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub path: PathBuf,
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
            if let Some(app) = parse_lnk(&path) {
                entries.push(app);
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

fn parse_lnk(lnk_path: &Path) -> Option<AppEntry> {
    let name = lnk_path.file_stem()?.to_string_lossy().into_owned();
    match ShellLink::open(lnk_path, lnk::encoding::WINDOWS_1252) {
        Ok(link) => {
            let path = match link.link_target() {
                Some(t) if !t.is_empty() => PathBuf::from(t),
                _ => {
                    debug!("UWP/no-target lnk fallback: {}", lnk_path.display());
                    lnk_path.to_path_buf()
                }
            };
            Some(AppEntry { name, path })
        }
        Err(e) => {
            warn!("failed to parse .lnk {}: {}", lnk_path.display(), e);
            Some(AppEntry {
                name,
                path: lnk_path.to_path_buf(),
            })
        }
    }
}
