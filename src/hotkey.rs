use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

pub struct Manager {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
}

impl Manager {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("create hotkey manager")?;
        Ok(Self {
            manager,
            current: None,
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
}
