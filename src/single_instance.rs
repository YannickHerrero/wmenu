use anyhow::Result;

#[cfg(windows)]
pub fn ensure() -> Result<()> {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let name: Vec<u16> = "Local\\wmenu-singleton-mutex"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // HANDLE is a Copy pointer wrapper with no Drop impl, so we can let it
    // fall out of scope: the mutex stays in the process's handle table and
    // the OS releases it when wmenu exits.
    let _ = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        anyhow::bail!("another wmenu instance is already running");
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn ensure() -> Result<()> {
    Ok(())
}
