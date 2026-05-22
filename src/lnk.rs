//! Resolve a Windows `.lnk` shortcut to its target executable path.
//!
//! Used by the bindings settings page so that picking an indexed Start-Menu
//! shortcut for a `FocusOrLaunch` action writes the underlying `.exe` path
//! (which the focus-existing-window check needs to match window processes
//! against), not the `.lnk` path itself.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

#[cfg(windows)]
#[allow(dead_code)] // wired into the app_picker widget in a later commit
pub fn resolve_target(lnk: &Path) -> Result<PathBuf> {
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink};
    use windows::core::{Interface, PCWSTR};

    // CoInitializeEx is idempotent across calls on the same thread: it returns
    // S_FALSE if COM was already initialised on this thread, and only the
    // first init "owns" the runtime. We intentionally don't CoUninitialize —
    // eframe's UI thread stays alive for the life of the process.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
    }

    let shell_link: IShellLinkW =
        unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }?;
    let persist: IPersistFile = shell_link.cast()?;

    let path_w: Vec<u16> = lnk
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe { persist.Load(PCWSTR(path_w.as_ptr()), STGM_READ) }?;

    // Plenty of headroom; long paths cap at ~32k on Windows but Start-Menu
    // targets are nowhere near that.
    let mut buf = vec![0u16; 1024];
    let mut find_data = WIN32_FIND_DATAW::default();
    unsafe {
        shell_link.GetPath(
            buf.as_mut_slice(),
            &mut find_data as *mut _,
            SLGP_RAWPATH.0 as u32,
        )
    }?;

    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let target = String::from_utf16_lossy(&buf[..len]);
    if target.is_empty() {
        bail!("IShellLinkW::GetPath returned an empty target for {lnk:?}");
    }
    Ok(PathBuf::from(target))
}

#[cfg(not(windows))]
#[allow(dead_code)] // wired into the app_picker widget in a later commit
pub fn resolve_target(_lnk: &Path) -> Result<PathBuf> {
    bail!("lnk::resolve_target is only implemented on Windows")
}
