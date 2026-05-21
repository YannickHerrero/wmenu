use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tray_icon::menu::MenuEvent;

use crate::config::Config;
use crate::hotkey::Manager as HotkeyMgr;
use crate::index;
use crate::index::SharedIndex;
use crate::launch;
use crate::matcher::Engine;
use crate::mru::Mru;
use crate::tray::Tray;
use crate::ui::{launcher, settings, theme};

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
    pub was_focused: bool,
    pub hotkey_input: String,
    pub hotkey_error: Option<String>,
    pub window_styled: bool,
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
        let ctx_menu = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = menu_tx.send(event);
            ctx_menu.request_repaint();
        }));

        theme::apply(&ctx, cfg.theme);

        let hotkey_input = cfg.hotkey.0.clone();
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
            was_focused: false,
            hotkey_input,
            hotkey_error: None,
            window_styled: false,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.maybe_rescan();
        let pos = center_position();
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.visible = true;
        self.focus_request = true;
        self.view = View::Launcher;
        self.query.clear();
        self.selected = 0;
        self.hotkey.set_escape_active(true);
    }

    fn maybe_rescan(&self) {
        let max_age = Duration::from_secs(self.cfg.scan_interval_minutes * 60);
        let stale = match self.index.load().scanned_at {
            None => true,
            Some(at) => at.elapsed() > max_age,
        };
        if stale {
            index::spawn_scan(self.index.clone(), self.cfg.extra_dirs.clone());
        }
    }

    fn hide(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.visible = false;
        self.was_focused = false;
        self.hotkey.set_escape_active(false);
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
            let id = event.id();
            if Some(id) == self.hotkey.current_id() {
                if self.visible {
                    self.hide(ctx);
                } else {
                    self.show(ctx);
                }
            } else if Some(id) == self.hotkey.escape_id() && self.visible {
                match self.view {
                    View::Launcher => self.hide(ctx),
                    View::Settings => {
                        self.view = View::Launcher;
                        self.focus_request = true;
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::palette(self.cfg.theme)
            .paper
            .to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if !self.window_styled {
            apply_window_style(frame);
            self.window_styled = true;
        }

        self.poll_tray(ui.ctx());
        self.poll_hotkey(ui.ctx());

        if self.visible {
            let focused = ui.ctx().input(|i| i.focused);
            if focused {
                self.was_focused = true;
            } else if self.was_focused {
                self.hide(ui.ctx());
                return;
            }
        }

        match self.view {
            View::Launcher => {
                let snapshot = self.index.load();
                let palette = theme::palette(self.cfg.theme);
                let action = launcher::show(
                    ui,
                    &palette,
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
                    launcher::Action::OpenSettings => {
                        self.view = View::Settings;
                    }
                    launcher::Action::Hide => {
                        self.hide(ui.ctx());
                    }
                }
            }
            View::Settings => {
                let palette = theme::palette(self.cfg.theme);
                let action = settings::show(
                    ui,
                    &palette,
                    &mut self.cfg.theme,
                    &mut self.hotkey_input,
                    self.hotkey_error.as_deref(),
                );
                match action {
                    settings::Action::None => {}
                    settings::Action::Back => {
                        self.view = View::Launcher;
                        self.focus_request = true;
                    }
                    settings::Action::ThemeChanged(t) => {
                        theme::apply(ui.ctx(), t);
                        if let Err(e) = self.cfg.save() {
                            tracing::warn!("save config: {e}");
                        }
                    }
                    settings::Action::ApplyHotkey(spec) => match self.hotkey.set(&spec) {
                        Ok(_) => {
                            self.cfg.hotkey.0 = spec;
                            self.hotkey_error = None;
                            if let Err(e) = self.cfg.save() {
                                tracing::warn!("save config: {e}");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("apply hotkey: {e}");
                            self.hotkey_error = Some(format!("{e}"));
                        }
                    },
                }
            }
        }
    }
}

#[cfg(windows)]
fn active_monitor_rect() -> (f32, f32, f32, f32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    };

    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        if GetCursorPos(&mut pt).is_err() {
            let w = GetSystemMetrics(SM_CXSCREEN) as f32;
            let h = GetSystemMetrics(SM_CYSCREEN) as f32;
            return (0.0, 0.0, w, h);
        }
    }
    let monitor: HMONITOR = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let ok = unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() };
    if !ok {
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as f32;
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as f32;
        return (0.0, 0.0, w, h);
    }
    let r = info.rcWork;
    (
        r.left as f32,
        r.top as f32,
        (r.right - r.left) as f32,
        (r.bottom - r.top) as f32,
    )
}

#[cfg(not(windows))]
fn active_monitor_rect() -> (f32, f32, f32, f32) {
    (0.0, 0.0, 1920.0, 1080.0)
}

fn center_position() -> egui::Pos2 {
    let (x, y, w, h) = active_monitor_rect();
    egui::pos2(x + (w - WINDOW_W) / 2.0, y + (h - WINDOW_H) / 2.0)
}

#[cfg(windows)]
fn apply_window_style(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_TOOLWINDOW,
    };

    let Ok(handle) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(win32.hwnd.get() as *mut _);
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current | (WS_EX_TOOLWINDOW.0 as isize));
    }
}

#[cfg(not(windows))]
fn apply_window_style(_frame: &eframe::Frame) {}
