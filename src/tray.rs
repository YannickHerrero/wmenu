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
    const SIZE: u32 = 32;
    const ACCENT: [u8; 4] = [0xB5, 0x59, 0x3A, 0xFF];
    const PAPER: [u8; 4] = [0xF4, 0xEB, 0xD9, 0xFF];
    const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

    let s = SIZE as f32;
    let cx = (s - 1.0) / 2.0;
    let cy = (s - 1.0) / 2.0;
    let outer_r = s * 0.48;
    let inner_r = s * 0.24;

    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let i = ((y * SIZE + x) * 4) as usize;
            let pixel = if d <= inner_r {
                PAPER
            } else if d <= outer_r {
                ACCENT
            } else {
                TRANSPARENT
            };
            rgba[i..i + 4].copy_from_slice(&pixel);
        }
    }
    Ok(Icon::from_rgba(rgba, SIZE, SIZE)?)
}
