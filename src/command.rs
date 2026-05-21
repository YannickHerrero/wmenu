use anyhow::{Context, Result};

#[cfg(windows)]
pub fn run(cmd: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // `cmd /c start "" <cmd>` routes through ShellExecute, which uses the
    // "App Paths" registry (so bare names like `firefox` resolve) and
    // handles .lnk activation. `cmd /c <cmd>` would skip both.
    let line = format!("start \"\" {cmd}");
    std::process::Command::new("cmd")
        .args(["/c", &line])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .with_context(|| format!("spawn `cmd /c {line}`"))?;
    tracing::debug!("ran binding command: {cmd}");
    Ok(())
}

#[cfg(not(windows))]
pub fn run(_cmd: &str) -> Result<()> {
    anyhow::bail!("command::run is only implemented on Windows")
}
