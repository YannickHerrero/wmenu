use eframe::egui;

use crate::config::Config;
use crate::index::SharedIndex;
use crate::matcher::Engine;
use crate::mru::Mru;

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
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.label("wmenu");
    }
}
