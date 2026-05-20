use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::config;

pub fn log_dir() -> Result<PathBuf> {
    Ok(config::project_dir()?.join("logs"))
}

pub fn init() -> Result<WorkerGuard> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("create log dir: {}", dir.display()))?;
    let file_appender = tracing_appender::rolling::daily(&dir, "wmenu.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_env("WMENU_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
    Ok(guard)
}
