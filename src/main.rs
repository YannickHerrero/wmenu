use anyhow::Result;

mod config;
mod index;
mod logging;

fn main() -> Result<()> {
    let _log_guard = logging::init()?;
    tracing::info!("wmenu starting");
    let _cfg = config::Config::load()?;
    Ok(())
}
