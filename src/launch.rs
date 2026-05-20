use std::path::Path;

use anyhow::Result;

#[cfg(windows)]
pub fn launch(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::PCWSTR;

    let op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let path_w: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        ShellExecuteW(
            Some(HWND(std::ptr::null_mut())),
            PCWSTR(op.as_ptr()),
            PCWSTR(path_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        anyhow::bail!("ShellExecuteW failed (code {})", result.0 as isize);
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn launch(_path: &Path) -> Result<()> {
    anyhow::bail!("launch is only implemented on Windows")
}
