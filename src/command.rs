use anyhow::{Context, Result};

#[cfg(windows)]
pub fn run(cmd: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // `cmd /c start "" <cmd>` routes through ShellExecute, which uses the
    // "App Paths" registry (so bare names like `firefox` resolve) and
    // handles .lnk activation. `cmd /c <cmd>` would skip both.
    //
    // raw_arg is required: Rust's normal arg quoting uses \" to escape
    // embedded quotes (MS C runtime style), but cmd.exe's parser doesn't
    // understand that and sees the backslashes literally.
    let line = format!("start \"\" {cmd}");
    std::process::Command::new("cmd")
        .arg("/c")
        .raw_arg(&line)
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
