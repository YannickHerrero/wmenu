use anyhow::{Context, Result};
use tray_icon::menu::{Menu, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct Tray {
    pub _tray: TrayIcon,
    pub show_id: MenuId,
    pub settings_id: MenuId,
    pub quit_id: MenuId,
}

pub fn build() -> Result<Tray> {
    let show = MenuItem::new("Show", true, None);
    let settings = MenuItem::new("Settings", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let show_id = show.id().clone();
    let settings_id = settings.id().clone();
    let quit_id = quit.id().clone();

    let menu = Menu::new();
    menu.append(&show).context("append Show")?;
    menu.append(&settings).context("append Settings")?;
    menu.append(&quit).context("append Quit")?;

    let icon = make_icon().context("build tray icon")?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("wmenu")
        .with_icon(icon)
        .build()
        .context("build tray icon")?;

    Ok(Tray {
        _tray: tray,
        show_id,
        settings_id,
        quit_id,
    })
}

fn make_icon() -> Result<Icon> {
    const SIZE: u32 = 16;
    const ACCENT: [u8; 4] = [0xB5, 0x59, 0x3A, 0xFF];
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for _ in 0..(SIZE * SIZE) {
        rgba.extend_from_slice(&ACCENT);
    }
    Ok(Icon::from_rgba(rgba, SIZE, SIZE)?)
}
