use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use notify::RecommendedWatcher;
use tray_icon::menu::MenuEvent;

use crate::action;
use crate::amphetamine::Amphetamine;
use crate::autostart;
use crate::config::{Config, watcher as config_watcher};
use crate::config::watcher::LastWritten;
use crate::hotkey::{BindingError, Manager as HotkeyMgr};
use crate::index;
use crate::index::SharedIndex;
use crate::ipc::IpcCommand;
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
    /// See [`LastWritten`] — used to suppress watcher feedback when the app
    /// auto-saves its own changes.
    pub last_written_config: LastWritten,
    _watcher: Option<RecommendedWatcher>,
    pub settings_open: bool,
    /// Cache of the last `settings_borderless` value we applied to the
    /// settings HWND via Win32, so we don't hammer EnumWindows every frame
    /// when nothing changed. Reset when the window closes.
    pub last_settings_chrome: Option<bool>,
    pub settings_page: settings::Page,
    /// Timestamp of the most-recent config mutation that hasn't been flushed
    /// to disk yet. The settings render loop saves once this is older than
    /// the debounce window.
    pub last_edit_at: Option<Instant>,
    /// Timestamp of the most-recent successful save. Used to fade out the
    /// "Saved" pill in the header.
    pub last_saved_at: Option<Instant>,
    /// Sticky error from the most-recent failed save. Cleared on the next
    /// successful save.
    pub last_save_error: Option<String>,
    pub settings_search: String,
    pub settings_search_focus_request: bool,
    /// One-shot focus hand-off set by a search-result click. The next render
    /// of the matching page should call `request_focus()` on the widget whose
    /// id matches this string, then clear the field.
    pub focus_target: Option<&'static str>,
    pub binding_errors: Vec<BindingError>,
    /// Receive half of the IPC channel populated by `ipc::spawn`. None when
    /// the listener failed to bind (port already in use, etc.).
    pub ipc_rx: Option<Receiver<IpcCommand>>,
}

impl App {
    pub fn new(
        cfg: Config,
        index: SharedIndex,
        mru: Mru,
        tray: Tray,
        mut hotkey: HotkeyMgr,
        ctx: egui::Context,
        ipc_rx: Option<Receiver<IpcCommand>>,
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
        let last_written_config = config_watcher::make_last_written();
        let watcher = match config_watcher::spawn(cfg_tx, last_written_config.clone()) {
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
            last_written_config,
            _watcher: watcher,
            settings_open: false,
            last_settings_chrome: None,
            settings_page: settings::Page::default(),
            last_edit_at: None,
            last_saved_at: None,
            last_save_error: None,
            settings_search: String::new(),
            settings_search_focus_request: false,
            focus_target: None,
            binding_errors,
            ipc_rx,
        }
    }

    /// Serialize and write the current config to disk, recording the
    /// serialized text in `last_written_config` so the file watcher can skip
    /// the subsequent self-write notification.
    pub fn save_config(&self) -> anyhow::Result<()> {
        use anyhow::Context as _;
        let text = toml::to_string_pretty(&self.cfg).context("serialize config")?;
        if let Ok(mut guard) = self.last_written_config.lock() {
            *guard = Some(text.clone());
        }
        let path = crate::config::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create config dir: {}", parent.display()))?;
        }
        std::fs::write(&path, &text)
            .with_context(|| format!("write config: {}", path.display()))?;
        Ok(())
    }

    pub(crate) fn apply_reloaded(&mut self, ctx: &egui::Context) {
        theme::apply(ctx, self.cfg.theme);
        // The settings UI owns hotkey_input / omakase_hotkey_input as
        // user-typed buffers; we don't sync them back from cfg here so that a
        // mid-typing apply (after auto-save) doesn't trample whatever the
        // user is still composing.
        match self.hotkey.set(&self.cfg.launcher.hotkey.0) {
            Ok(_) => self.hotkey_error = None,
            Err(e) => {
                tracing::warn!("re-apply launcher hotkey: {e}");
                self.hotkey_error = Some(format!("{e}"));
            }
        }
        match self.hotkey.set_omakase(&self.cfg.launcher.omakase_hotkey.0) {
            Ok(_) => self.omakase_hotkey_error = None,
            Err(e) => {
                tracing::warn!("re-apply omakase hotkey: {e}");
                self.omakase_hotkey_error = Some(format!("{e}"));
            }
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

    /// Drain any pending IPC commands and apply them. Currently just
    /// SetTheme; the same drain pattern accommodates future commands.
    fn poll_ipc(&mut self, ctx: &egui::Context) {
        // Drain into a Vec first so the immutable borrow on self.ipc_rx
        // ends before we call &mut self methods (apply_reloaded /
        // save_config) inside the dispatch loop.
        let pending: Vec<IpcCommand> = if let Some(rx) = &self.ipc_rx {
            std::iter::from_fn(|| rx.try_recv().ok()).collect()
        } else {
            Vec::new()
        };
        for cmd in pending {
            match cmd {
                IpcCommand::SetTheme(theme) => {
                    tracing::info!(?theme, "switching theme via ipc");
                    self.cfg.theme = theme;
                    self.apply_reloaded(ctx);
                    if let Err(e) = self.save_config() {
                        tracing::warn!("save after ipc set-theme: {e}");
                    }
                }
            }
        }
    }

    fn render_settings_viewport(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let borderless = self.cfg.settings_borderless;
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

        // Strip / restore WS_CAPTION on the settings HWND so we get a
        // wezterm-style "RESIZE only" chrome (no titlebar but resize border
        // stays, which keeps the window floatable / sizable in tiling WMs
        // like GlazeWM). Only fired when the desired state changes — finding
        // the HWND is a Win32 EnumWindows call.
        if self.last_settings_chrome != Some(borderless)
            && apply_settings_chrome(borderless)
        {
            self.last_settings_chrome = Some(borderless);
        }
        if close_requested {
            // Flush any pending debounced edit so closing the window can't
            // drop unsaved changes.
            if self.last_edit_at.is_some()
                && let Err(e) = self.save_config()
            {
                tracing::warn!("flush on close: {e}");
            }
            self.last_edit_at = None;
            self.settings_open = false;
            // Next open will recreate the HWND, so forget the cached chrome
            // state so we re-apply on the new window.
            self.last_settings_chrome = None;
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
        self.poll_ipc(ui.ctx());

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
                        if let Err(e) = self.save_config() {
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

/// Toggle the wezterm-style "RESIZE" chrome on the settings window: strip
/// `WS_CAPTION` (titlebar + system menu controls) while keeping
/// `WS_THICKFRAME` (the resize border). Looks the window up by its title via
/// `EnumWindows` filtered to our own process. Returns `true` if the matching
/// HWND was found and updated this frame; the caller uses that to gate the
/// "already applied" cache so we keep retrying until the HWND exists.
///
/// Also installs a `WM_NCCALCSIZE` subclass procedure on the settings HWND
/// that, when borderless is on, claims the top non-client strip as client
/// area — otherwise Win32 reserves a few pixels at the top for what would
/// have been the caption bar and DWM paints them white. Top-edge resize is
/// the cost of that fix (corners + the other three edges still resize). The
/// subclass is installed once per HWND; toggling borderless just flips the
/// atomic the subclass reads.
#[cfg(windows)]
fn apply_settings_chrome(borderless: bool) -> bool {
    use std::cell::Cell;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, TRUE, WPARAM};
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_STYLE, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, HWND_TOP, NCCALCSIZE_PARAMS, SWP_FRAMECHANGED, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WM_NCCALCSIZE, WS_CAPTION,
    };
    use windows::core::BOOL;

    const TITLE: &str = "wmenu — settings";
    const SUBCLASS_ID: usize = 0x77_6D_65_6E; // 'wmen' — distinguishes our subclass

    static BORDERLESS: AtomicBool = AtomicBool::new(false);
    static SUBCLASSED_HWND: AtomicIsize = AtomicIsize::new(0);

    /// Subclass procedure: when borderless is on, restore the top edge of
    /// the proposed client rect to match the window rect, eating the strip
    /// Win32 would otherwise reserve for the (absent) titlebar. All other
    /// messages and the non-borderless case fall through to default
    /// handling so the rest of the window keeps standard non-client behaviour
    /// (resize on three remaining edges + four corners, hit-test, etc.).
    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _ref_data: usize,
    ) -> LRESULT {
        if msg == WM_NCCALCSIZE && wparam.0 != 0 && BORDERLESS.load(Ordering::Relaxed) {
            unsafe {
                let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
                let original_top = params.rgrc[0].top;
                let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                params.rgrc[0].top = original_top;
                return result;
            }
        }
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }

    thread_local! {
        static MATCH: Cell<isize> = const { Cell::new(0) };
        static OWN_PID: Cell<u32> = const { Cell::new(0) };
    }

    unsafe extern "system" fn cb(hwnd: HWND, _: LPARAM) -> BOOL {
        unsafe {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid != OWN_PID.with(|p| p.get()) {
                return TRUE;
            }
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return TRUE;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let got = GetWindowTextW(hwnd, &mut buf);
            if got <= 0 {
                return TRUE;
            }
            let title = String::from_utf16_lossy(&buf[..got as usize]);
            if title == TITLE {
                MATCH.with(|m| m.set(hwnd.0 as isize));
                return BOOL(0); // stop enumeration
            }
        }
        TRUE
    }

    OWN_PID.with(|p| p.set(unsafe { GetCurrentProcessId() }));
    MATCH.with(|m| m.set(0));
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(0));
    }
    let hwnd_ptr = MATCH.with(|m| m.get());
    if hwnd_ptr == 0 {
        return false;
    }
    let hwnd = HWND(hwnd_ptr as *mut _);

    // Sync the atomic before SWP_FRAMECHANGED so the subclass sees the right
    // state on the recompute pass.
    BORDERLESS.store(borderless, Ordering::Relaxed);

    // Install the subclass once per HWND. SetWindowSubclass with a stable id
    // is idempotent for the same HWND, but if the user closed + reopened
    // settings, the new HWND needs its own install pass.
    if SUBCLASSED_HWND.load(Ordering::Relaxed) != hwnd_ptr {
        unsafe {
            let _ = SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0);
        }
        SUBCLASSED_HWND.store(hwnd_ptr, Ordering::Relaxed);
    }

    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let caption_bits = WS_CAPTION.0 as isize;
        let new_style = if borderless {
            style & !caption_bits
        } else {
            style | caption_bits
        };
        if new_style != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);
        }
        // Always trigger a non-client recompute on toggle so the subclass'
        // top-edge override (or the lack of it) takes effect immediately.
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }
    true
}

#[cfg(not(windows))]
fn apply_settings_chrome(_borderless: bool) -> bool {
    true
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
