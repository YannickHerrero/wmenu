use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use tracing::warn;

pub struct Manager {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
    escape: Option<HotKey>,
}

impl Manager {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("create hotkey manager")?;
        Ok(Self {
            manager,
            current: None,
            escape: None,
        })
    }

    pub fn set(&mut self, spec: &str) -> Result<u32> {
        if let Some(old) = self.current.take() {
            let _ = self.manager.unregister(old);
        }
        let hotkey: HotKey = spec
            .parse()
            .with_context(|| format!("parse hotkey: {spec}"))?;
        self.manager
            .register(hotkey)
            .with_context(|| format!("register hotkey: {spec}"))?;
        let id = hotkey.id();
        self.current = Some(hotkey);
        Ok(id)
    }

    pub fn current_id(&self) -> Option<u32> {
        self.current.map(|h| h.id())
    }

    pub fn set_escape_active(&mut self, active: bool) {
        if active {
            if self.escape.is_some() {
                return;
            }
            let hotkey: HotKey = match "Escape".parse() {
                Ok(h) => h,
                Err(e) => {
                    warn!("parse Escape hotkey: {e}");
                    return;
                }
            };
            if let Err(e) = self.manager.register(hotkey) {
                warn!("register Escape hotkey: {e}");
                return;
            }
            self.escape = Some(hotkey);
        } else if let Some(old) = self.escape.take() {
            let _ = self.manager.unregister(old);
        }
    }

    pub fn escape_id(&self) -> Option<u32> {
        self.escape.map(|h| h.id())
    }
}
