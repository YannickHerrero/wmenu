use eframe::egui;

use tray_icon::menu::MenuEvent;

use crate::config::Config;
use crate::hotkey::Manager as HotkeyMgr;
use crate::index::SharedIndex;
use crate::launch;
use crate::matcher::Engine;
use crate::mru::Mru;
use crate::tray::Tray;
use crate::ui::launcher;

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
    pub view: View,
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub focus_request: bool,
}

impl App {
    pub fn new(cfg: Config, index: SharedIndex, mru: Mru, tray: Tray, hotkey: HotkeyMgr) -> Self {
        Self {
            cfg,
            index,
            mru,
            matcher: Engine::new(),
            tray,
            hotkey,
            view: View::Launcher,
            visible: false,
            query: String::new(),
            selected: 0,
            focus_request: false,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
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
        while let Ok(event) = MenuEvent::receiver().try_recv() {
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
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_tray(ui.ctx());

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
