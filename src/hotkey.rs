use std::collections::HashMap;

use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use tracing::warn;

use crate::config::Binding;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BindingError {
    pub index: usize,
    pub message: String,
}

struct Registered {
    hotkey: HotKey,
    binding_index: usize,
}

pub struct Manager {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
    escape: Option<HotKey>,
    omakase: Option<HotKey>,
    bindings: HashMap<u32, Registered>,
}

impl Manager {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("create hotkey manager")?;
        Ok(Self {
            manager,
            current: None,
            escape: None,
            omakase: None,
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

    pub fn set_omakase(&mut self, spec: &str) -> Result<u32> {
        if let Some(old) = self.omakase.take() {
            let _ = self.manager.unregister(old);
        }
        let hotkey: HotKey = spec
            .parse()
            .with_context(|| format!("parse omakase hotkey: {spec}"))?;
        self.manager
            .register(hotkey)
            .with_context(|| format!("register omakase hotkey: {spec}"))?;
        let id = hotkey.id();
        self.omakase = Some(hotkey);
        Ok(id)
    }

    pub fn omakase_id(&self) -> Option<u32> {
        self.omakase.map(|h| h.id())
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

    /// Reserved hotkey ids the user can't override via bindings.
    fn reserved_ids(&self) -> [Option<u32>; 3] {
        [
            self.current.map(|h| h.id()),
            self.omakase.map(|h| h.id()),
            self.escape.map(|h| h.id()),
        ]
    }

    #[allow(dead_code)]
    pub fn set_bindings(&mut self, list: &[Binding]) -> Vec<BindingError> {
        for (_, reg) in self.bindings.drain() {
            let _ = self.manager.unregister(reg.hotkey);
        }

        let reserved = self.reserved_ids();
        let mut errors = Vec::new();
        let mut seen: HashMap<u32, usize> = HashMap::new();

        for (index, binding) in list.iter().enumerate() {
            let trimmed = binding.key.trim();
            if trimmed.is_empty() {
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
            if reserved.iter().any(|r| *r == Some(hotkey.id())) {
                errors.push(BindingError {
                    index,
                    message: format!("'{trimmed}' is reserved by a built-in hotkey"),
                });
                continue;
            }
            if let Some(prev) = seen.get(&hotkey.id()) {
                errors.push(BindingError {
                    index,
                    message: format!("duplicate of binding #{prev}"),
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
            seen.insert(hotkey.id(), index);
            self.bindings.insert(
                hotkey.id(),
                Registered {
                    hotkey,
                    binding_index: index,
                },
            );
            tracing::info!(
                "registered binding #{} '{}' ({})",
                index,
                binding.label,
                trimmed
            );
        }

        if !errors.is_empty() {
            tracing::warn!(
                "{} of {} binding(s) failed to register",
                errors.len(),
                list.len()
            );
        }
        errors
    }

    #[allow(dead_code)]
    pub fn binding_index_for(&self, id: u32) -> Option<usize> {
        self.bindings.get(&id).map(|b| b.binding_index)
    }
}
