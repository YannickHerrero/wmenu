# wmenu

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

Run `wmenu.exe`. A tray icon appears; press the global hotkey (default `Shift+Space`) to open the launcher, type to filter, `↑/↓` to navigate, `Enter` to launch, `Esc` to dismiss. The window also dismisses on focus loss.

Right-click the tray icon for `Show`, `Settings`, `Quit`. Settings lets you switch theme (Paper / Stone / Sage / Clay / Ink) and rebind the hotkey live.

## State

- Config: `%APPDATA%\wmenu\config.toml`
- MRU history: `%APPDATA%\wmenu\mru.json`
- Logs: `%APPDATA%\wmenu\logs\wmenu.log` (daily rolling)

Set `WMENU_LOG=debug` to widen the log filter.

## Indexing

On start the daemon scans the per-user and system Start Menu directories for `.lnk` shortcuts. The index refreshes when the hotkey fires if the last scan is older than `scan_interval_minutes` (default 5). UWP/Store shortcuts that don't resolve to a target path are launched via the `.lnk` itself.
