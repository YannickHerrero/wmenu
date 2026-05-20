use eframe::egui;

use crate::config::Config;
use crate::index::SharedIndex;
use crate::launch;
use crate::matcher::Engine;
use crate::mru::Mru;
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
    pub view: View,
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub focus_request: bool,
}

impl App {
    pub fn new(cfg: Config, index: SharedIndex, mru: Mru) -> Self {
        Self {
            cfg,
            index,
            mru,
            matcher: Engine::new(),
            view: View::Launcher,
            visible: false,
            query: String::new(),
            selected: 0,
            focus_request: false,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
