use anyhow::Result;

mod config;
mod index;
mod logging;

fn main() -> Result<()> {
    let _log_guard = logging::init()?;
    tracing::info!("wmenu starting");
    let cfg = config::Config::load()?;
    let shared_index = index::new_shared();
    index::spawn_scan(shared_index.clone(), cfg.extra_dirs.clone());
    let _ = (cfg, shared_index);
    Ok(())
}
