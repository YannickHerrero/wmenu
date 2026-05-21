use std::collections::HashMap;

use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use tracing::warn;

use crate::config::HotkeyBinding;

pub struct RegisteredBinding {
    pub hotkey: HotKey,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct BindingError {
    pub index: usize,
    pub message: String,
}

pub struct Manager {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
    escape: Option<HotKey>,
    bindings: HashMap<u32, RegisteredBinding>,
}

impl Manager {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("create hotkey manager")?;
        Ok(Self {
            manager,
            current: None,
            escape: None,
            bindings: HashMap::new(),
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

    pub fn set_bindings(&mut self, list: &[HotkeyBinding]) -> Vec<BindingError> {
        for (_, reg) in self.bindings.drain() {
            let _ = self.manager.unregister(reg.hotkey);
        }

        let mut errors = Vec::new();
        for (index, binding) in list.iter().enumerate() {
            let trimmed = binding.spec.trim();
            if trimmed.is_empty() || binding.command.trim().is_empty() {
                continue;
            }
            let hotkey: HotKey = match trimmed.parse() {
                Ok(h) => h,
                Err(e) => {
                    errors.push(BindingError {
                        index,
                        message: format!("invalid hotkey '{trimmed}': {e}"),
                    });
                    continue;
                }
            };
            if Some(hotkey.id()) == self.current.map(|h| h.id()) {
                errors.push(BindingError {
                    index,
                    message: "conflicts with launcher hotkey".to_string(),
                });
                continue;
            }
            if let Err(e) = self.manager.register(hotkey) {
                errors.push(BindingError {
                    index,
                    message: format!("register failed: {e}"),
                });
                continue;
            }
            self.bindings.insert(
                hotkey.id(),
                RegisteredBinding {
                    hotkey,
                    command: binding.command.clone(),
                },
            );
        }
        errors
    }

    pub fn command_for(&self, id: u32) -> Option<&str> {
        self.bindings.get(&id).map(|b| b.command.as_str())
    }
}
