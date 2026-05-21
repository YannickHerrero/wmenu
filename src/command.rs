use anyhow::{Context, Result};

#[cfg(windows)]
pub fn run(cmd: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    std::process::Command::new("cmd")
        .args(["/c", cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .with_context(|| format!("spawn `cmd /c {cmd}`"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn run(_cmd: &str) -> Result<()> {
    anyhow::bail!("command::run is only implemented on Windows")
}
