use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Top,
    System,
    Theme,
    Confirm(SystemAction),
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAction {
    Shutdown,
    Restart,
    Hibernate,
}

impl SystemAction {
    pub fn label(self) -> &'static str {
        match self {
            SystemAction::Shutdown => "Shut down",
            SystemAction::Restart => "Restart",
            SystemAction::Hibernate => "Hibernate",
        }
    }
}

#[cfg(windows)]
pub fn execute_system(action: SystemAction) -> Result<()> {
    let flag = match action {
        SystemAction::Shutdown => "/s",
        SystemAction::Restart => "/r",
        SystemAction::Hibernate => "/h",
    };
    std::process::Command::new("shutdown")
        .args([flag, "/t", "0"])
        .spawn()
        .with_context(|| format!("spawn `shutdown {flag} /t 0`"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn execute_system(_action: SystemAction) -> Result<()> {
    anyhow::bail!("system actions are only implemented on Windows")
}
