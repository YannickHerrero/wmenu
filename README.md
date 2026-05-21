# wmenu

![wmenu demo](docs/wmenu-demo.webp)

Keyboard-driven app launcher for Windows. Modeled after dmenu: tray-resident, summoned by a global hotkey, fuzzy-matches Start Menu apps, launches via Enter.

## Build

Requires the Rust toolchain (1.85+ for edition 2024).

```
cargo build --release
```

On non-Windows hosts, cross-compile against the Windows target:

```
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

The build needs `mingw-w64` (Debian/Ubuntu: `sudo apt install mingw-w64`).

## Use

Run `wmenu.exe`. A tray icon appears; press the global hotkey (default `Alt+Space`) to open the launcher, type to filter, `↑/↓` to navigate, `Enter` to launch, `Esc` to dismiss. The window also dismisses on focus loss.

Open the settings page from inside the launcher with `Ctrl+,` or via the tray menu's `Settings` entry. Settings lets you switch theme (Paper / Stone / Sage / Clay / Ink), rebind the launcher hotkey, and toggle launch-at-Windows-login. The entire page is keyboard-driven: Tab navigates, Space toggles, Enter activates.

## State

- Config: `%APPDATA%\wmenu\config\config.toml`
- MRU history: `%APPDATA%\wmenu\config\mru.json`
- Logs: `%APPDATA%\wmenu\config\logs\wmenu.log.<date>` (daily rolling)

Set `WMENU_LOG=debug` to widen the log filter.

## Indexing

On start the daemon scans the per-user and system Start Menu directories for `.lnk` shortcuts. The index refreshes when the hotkey fires if the last scan is older than `scan_interval_minutes` (default 5). Shortcuts are passed to `ShellExecuteW` as-is, so target arguments (e.g. Firefox web apps) and UWP activations work the same way they do from Start Menu.
