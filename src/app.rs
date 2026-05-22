use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use notify::RecommendedWatcher;
use tray_icon::menu::MenuEvent;

use crate::action;
use crate::amphetamine::Amphetamine;
use crate::autostart;
use crate::config::{Config, watcher as config_watcher};
use crate::hotkey::{BindingError, Manager as HotkeyMgr};
use crate::index;
use crate::index::SharedIndex;
use crate::launch;
use crate::matcher::Engine;
use crate::mru::Mru;
use crate::omakase;
use crate::tray::Tray;
use crate::ui::{launcher, omakase as ui_omakase, settings, theme};

pub const WINDOW_W: f32 = 640.0;
pub const WINDOW_H: f32 = 400.0;

pub enum View {
    Launcher,
    Omakase,
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
    pub amphetamine: Amphetamine,
    pub omakase_page: omakase::Page,
    pub omakase_query: String,
    pub omakase_selected: usize,
    pub omakase_focus_request: bool,
    pub omakase_hotkey_input: String,
    pub omakase_hotkey_error: Option<String>,
    pub cfg_rx: Receiver<Config>,
    _watcher: Option<RecommendedWatcher>,
    pub settings_open: bool,
    pub settings_page: settings::Page,
    pub settings_dirty: bool,
    pub settings_status: Option<String>,
    pub settings_search: String,
    pub settings_search_focus_request: bool,
    /// One-shot focus hand-off set by a search-result click. The next render
    /// of the matching page should call `request_focus()` on the widget whose
    /// id matches this string, then clear the field.
    pub focus_target: Option<&'static str>,
    pub binding_errors: Vec<BindingError>,
}

impl App {
    pub fn new(
        cfg: Config,
        index: SharedIndex,
        mru: Mru,
        tray: Tray,
        mut hotkey: HotkeyMgr,
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

        let (cfg_tx, cfg_rx) = channel();
        let watcher = match config_watcher::spawn(cfg_tx) {
            Ok(w) => Some(w),
            Err(e) => {
                tracing::warn!("start config watcher: {e}");
                None
            }
        };

        theme::apply(&ctx, cfg.theme);

        if let Err(e) = hotkey.set_omakase(&cfg.launcher.omakase_hotkey.0) {
            tracing::warn!(
                "register omakase hotkey {}: {e}",
                cfg.launcher.omakase_hotkey.0
            );
        }

        let binding_errors = hotkey.set_bindings(&cfg.bindings);
        for err in &binding_errors {
            tracing::warn!("binding #{}: {}", err.index, err.message);
        }

        let hotkey_input = cfg.launcher.hotkey.0.clone();
        let omakase_hotkey_input = cfg.launcher.omakase_hotkey.0.clone();
        let amphetamine = Amphetamine::new(cfg.amphetamine_enabled);
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
            amphetamine,
            omakase_page: omakase::Page::Top,
            omakase_query: String::new(),
            omakase_selected: 0,
            omakase_focus_request: false,
            omakase_hotkey_input,
            omakase_hotkey_error: None,
            cfg_rx,
            _watcher: watcher,
            settings_open: false,
            settings_page: settings::Page::default(),
            settings_dirty: false,
            settings_status: None,
            settings_search: String::new(),
            settings_search_focus_request: false,
            focus_target: None,
            binding_errors,
        }
    }

    pub(crate) fn apply_reloaded(&mut self, ctx: &egui::Context) {
        theme::apply(ctx, self.cfg.theme);
        if let Err(e) = self.hotkey.set(&self.cfg.launcher.hotkey.0) {
            tracing::warn!("re-apply launcher hotkey: {e}");
        } else {
            self.hotkey_input = self.cfg.launcher.hotkey.0.clone();
            self.hotkey_error = None;
        }
        if let Err(e) = self.hotkey.set_omakase(&self.cfg.launcher.omakase_hotkey.0) {
            tracing::warn!("re-apply omakase hotkey: {e}");
        } else {
            self.omakase_hotkey_input = self.cfg.launcher.omakase_hotkey.0.clone();
            self.omakase_hotkey_error = None;
        }
        let errs = self.hotkey.set_bindings(&self.cfg.bindings);
        for err in &errs {
            tracing::warn!("binding #{}: {}", err.index, err.message);
        }
        self.binding_errors = errs;
        self.amphetamine.set(self.cfg.amphetamine_enabled);
        if let Err(e) = autostart::sync(self.cfg.daemon.autostart) {
            tracing::warn!("sync autostart: {e}");
        }
    }

    fn poll_cfg(&mut self, ctx: &egui::Context) {
        while let Ok(cfg) = self.cfg_rx.try_recv() {
            self.cfg = cfg;
            self.apply_reloaded(ctx);
        }
    }

    fn render_settings_viewport(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let close_requested = ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("wmenu_settings"),
            egui::ViewportBuilder::default()
                .with_title("wmenu — settings")
                .with_inner_size([760.0, 560.0])
                .with_min_inner_size([520.0, 380.0]),
            |child_ctx, _class| {
                settings::render(self, child_ctx);
                child_ctx.input(|i| i.viewport().close_requested())
            },
        );
        if close_requested {
            self.settings_open = false;
        }
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        let pos = center_position();
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.visible = true;
        self.hotkey.set_escape_active(true);
    }

    fn show(&mut self, ctx: &egui::Context) {
        self.maybe_rescan();
        self.show_window(ctx);
        self.focus_request = true;
        self.view = View::Launcher;
        self.query.clear();
        self.selected = 0;
    }

    fn show_omakase(&mut self, ctx: &egui::Context) {
        self.show_window(ctx);
        self.view = View::Omakase;
        self.omakase_page = omakase::Page::Top;
        self.omakase_query.clear();
        self.omakase_selected = 0;
        self.omakase_focus_request = true;
    }

    fn maybe_rescan(&self) {
        let max_age = Duration::from_secs(self.cfg.launcher.scan_interval_minutes * 60);
        let stale = match self.index.load().scanned_at {
            None => true,
            Some(at) => at.elapsed() > max_age,
        };
        if stale {
            index::spawn_scan(self.index.clone(), self.cfg.launcher.extra_dirs.clone());
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
                self.settings_open = true;
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
            } else if Some(id) == self.hotkey.omakase_id() {
                if self.visible {
                    self.hide(ctx);
                } else {
                    self.show_omakase(ctx);
                }
            } else if Some(id) == self.hotkey.escape_id() && self.visible {
                match self.view {
                    View::Launcher => self.hide(ctx),
                    View::Omakase => self.omakase_back_or_hide(ctx),
                }
            } else if let Some(idx) = self.hotkey.binding_index_for(id)
                && let Some(binding) = self.cfg.bindings.get(idx)
            {
                tracing::info!("binding fired: '{}' -> {:?}", binding.label, binding.action);
                if let Err(e) = action::run(&binding.action) {
                    tracing::warn!("binding action failed: {e}");
                }
            }
        }
    }

    fn omakase_back_or_hide(&mut self, ctx: &egui::Context) {
        self.omakase_query.clear();
        self.omakase_selected = 0;
        self.omakase_focus_request = true;
        match self.omakase_page {
            omakase::Page::Top => self.hide(ctx),
            omakase::Page::System | omakase::Page::Help => {
                self.omakase_page = omakase::Page::Top;
            }
            omakase::Page::Confirm(_) => {
                self.omakase_page = omakase::Page::System;
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
            // Force-hide on the first frame: ViewportBuilder::with_visible(false)
            // sometimes leaks a brief flash on Windows, and the SetWindowLongPtrW
            // call in apply_window_style can trigger an unwanted redraw.
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.window_styled = true;
        }

        self.poll_tray(ui.ctx());
        self.poll_hotkey(ui.ctx());
        self.poll_cfg(ui.ctx());

        self.render_settings_viewport(ui.ctx());

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
                        self.settings_open = true;
                    }
                    launcher::Action::Hide => {
                        self.hide(ui.ctx());
                    }
                }
            }
            View::Omakase => {
                let palette = theme::palette(self.cfg.theme);
                let amph = self.amphetamine.is_enabled();
                let action = ui_omakase::show(
                    ui,
                    &palette,
                    self.omakase_page,
                    &mut self.omakase_query,
                    &mut self.omakase_selected,
                    amph,
                    self.omakase_focus_request,
                );
                self.omakase_focus_request = false;
                match action {
                    ui_omakase::Action::None => {}
                    ui_omakase::Action::Back => self.omakase_back_or_hide(ui.ctx()),
                    ui_omakase::Action::Hide => self.hide(ui.ctx()),
                    ui_omakase::Action::EnterSystem => {
                        self.omakase_page = omakase::Page::System;
                        self.omakase_query.clear();
                        self.omakase_selected = 0;
                        self.omakase_focus_request = true;
                    }
                    ui_omakase::Action::EnterHelp => {
                        self.omakase_page = omakase::Page::Help;
                    }
                    ui_omakase::Action::ToggleAmphetamine => {
                        let new = !self.amphetamine.is_enabled();
                        self.amphetamine.set(new);
                        self.cfg.amphetamine_enabled = new;
                        if let Err(e) = self.cfg.save() {
                            tracing::warn!("save config: {e}");
                        }
                    }
                    ui_omakase::Action::SelectSystem(action) => {
                        self.omakase_page = omakase::Page::Confirm(action);
                        self.omakase_focus_request = true;
                    }
                    ui_omakase::Action::ConfirmSystem(action) => {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        self.visible = false;
                        self.hotkey.set_escape_active(false);
                        if let Err(e) = omakase::execute_system(action) {
                            tracing::warn!("execute {:?}: {e}", action);
                        }
                    }
                }
            }
        }

        let accent = theme::palette(self.cfg.theme).accent;
        let rect = ui.ctx().content_rect();
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, accent),
            egui::StrokeKind::Inside,
        );
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
