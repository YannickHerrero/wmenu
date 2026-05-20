use std::sync::mpsc::{Receiver, channel};

use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tray_icon::menu::MenuEvent;

use crate::config::Config;
use crate::hotkey::Manager as HotkeyMgr;
use crate::index::SharedIndex;
use crate::launch;
use crate::matcher::Engine;
use crate::mru::Mru;
use crate::tray::Tray;
use crate::ui::launcher;

pub const WINDOW_W: f32 = 640.0;
pub const WINDOW_H: f32 = 400.0;

pub enum View {
    Launcher,
    Settings,
}

pub struct App {
    pub cfg: Config,
    pub index: SharedIndex,
    pub mru: Mru,
    pub matcher: Engine,
    pub tray: Tray,
    pub hotkey: HotkeyMgr,
    pub hotkey_rx: Receiver<GlobalHotKeyEvent>,
    pub menu_rx: Receiver<MenuEvent>,
    pub view: View,
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub focus_request: bool,
}

impl App {
    pub fn new(
        cfg: Config,
        index: SharedIndex,
        mru: Mru,
        tray: Tray,
        hotkey: HotkeyMgr,
        ctx: egui::Context,
    ) -> Self {
        let (hotkey_tx, hotkey_rx) = channel();
        let ctx_hk = ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event| {
            let _ = hotkey_tx.send(event);
            ctx_hk.request_repaint();
        }));

        let (menu_tx, menu_rx) = channel();
        let ctx_menu = ctx;
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = menu_tx.send(event);
            ctx_menu.request_repaint();
        }));

        Self {
            cfg,
            index,
            mru,
            matcher: Engine::new(),
            tray,
            hotkey,
            hotkey_rx,
            menu_rx,
            view: View::Launcher,
            visible: false,
            query: String::new(),
            selected: 0,
            focus_request: false,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        let pos = center_position();
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.visible = true;
        self.focus_request = true;
        self.view = View::Launcher;
        self.query.clear();
        self.selected = 0;
    }

    fn hide(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.visible = false;
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.menu_rx.try_recv() {
            if event.id == self.tray.show_id {
                self.show(ctx);
            } else if event.id == self.tray.settings_id {
                self.show(ctx);
                self.view = View::Settings;
            } else if event.id == self.tray.quit_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn poll_hotkey(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.hotkey_rx.try_recv() {
            if event.state() != HotKeyState::Pressed {
                continue;
            }
            if Some(event.id()) != self.hotkey.current_id() {
                continue;
            }
            if self.visible {
                self.hide(ctx);
            } else {
                self.show(ctx);
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_tray(ui.ctx());
        self.poll_hotkey(ui.ctx());

        match self.view {
            View::Launcher => {
                let snapshot = self.index.load();
                let action = launcher::show(
                    ui,
                    &mut self.query,
                    &mut self.selected,
                    &snapshot.entries,
                    &mut self.matcher,
                    &self.mru,
                    self.focus_request,
                );
                self.focus_request = false;
                match action {
                    launcher::Action::None => {}
                    launcher::Action::Launch(idx) => {
                        let path = snapshot.entries[idx].path.clone();
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        self.visible = false;
                        match launch::launch(&path) {
                            Ok(()) => {
                                self.mru.record_launch(&path);
                                if let Err(e) = self.mru.save() {
                                    tracing::warn!("save mru: {e}");
                                }
                            }
                            Err(e) => tracing::warn!("launch {}: {e}", path.display()),
                        }
                        self.query.clear();
                        self.selected = 0;
                    }
                    launcher::Action::Hide => {
                        // hide wiring lands in step 15
                    }
                }
            }
            View::Settings => {
                ui.label("Settings (todo)");
            }
        }
    }
}

#[cfg(windows)]
fn primary_monitor_size() -> (f32, f32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as f32;
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as f32;
    (w.max(WINDOW_W), h.max(WINDOW_H))
}

#[cfg(not(windows))]
fn primary_monitor_size() -> (f32, f32) {
    (1920.0, 1080.0)
}

fn center_position() -> egui::Pos2 {
    let (sw, sh) = primary_monitor_size();
    egui::pos2((sw - WINDOW_W) / 2.0, (sh - WINDOW_H) / 2.0)
}
