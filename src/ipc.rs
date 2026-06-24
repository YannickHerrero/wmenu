//! Tiny TCP control server. The running wmenu instance listens on
//! 127.0.0.1:17129 and accepts one-line commands so external tools (AHK
//! scripts, glazewm keybinds, terminal one-liners) can change theme
//! without opening the settings window.
//!
//! Mirrors the IPC shape in the sibling `wbar` project (port 17128) so a
//! single hotkey can switch theme across both tools at once.
//!
//! Protocol: one connection, one line, one response. The CLI client
//! (`wmenu set-theme Ink`) lives in main.rs and just opens a stream,
//! writes a line, reads the reply.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result};
use eframe::egui;

use crate::config::Theme;

/// TCP port the control server binds on. Loopback-only; not configurable
/// in v1 — picked one above wbar's 17128 so the two tools coexist without
/// thinking about port allocation.
pub const PORT: u16 = 17129;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    SetTheme(Theme),
    RotateWallpaper,
}

impl IpcCommand {
    /// Parse a single line from the wire. Whitespace-trimmed, lowercased
    /// command word, optional space-separated argument.
    pub fn parse(line: &str) -> Result<Self, String> {
        let line = line.trim();
        if line.is_empty() {
            return Err("empty command".into());
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let cmd = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("").trim();
        match cmd {
            "set-theme" => {
                if arg.is_empty() {
                    Err("set-theme requires a theme name".into())
                } else {
                    Theme::from_str(arg).map(Self::SetTheme)
                }
            }
            "rotate-wallpaper" => Ok(Self::RotateWallpaper),
            other => Err(format!("unknown command: {other:?}")),
        }
    }
}

/// Spawn the listener thread. Returns the receive half of the channel that
/// `App` drains each frame.
pub fn spawn(ctx: egui::Context) -> Result<Receiver<IpcCommand>> {
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), PORT);
    let listener = TcpListener::bind(addr).with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "ipc listener bound");

    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("wmenu-ipc".into())
        .spawn(move || {
            for conn in listener.incoming() {
                match conn {
                    Ok(stream) => {
                        if let Err(err) = handle_connection(stream, &tx, &ctx) {
                            tracing::debug!(?err, "ipc connection error");
                        }
                    }
                    Err(err) => tracing::warn!(?err, "ipc accept failed"),
                }
            }
        })
        .context("spawning ipc thread")?;
    Ok(rx)
}

fn handle_connection(
    mut stream: TcpStream,
    tx: &mpsc::Sender<IpcCommand>,
    ctx: &egui::Context,
) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let response = match IpcCommand::parse(&line) {
        Ok(cmd) => {
            tracing::info!(?cmd, "ipc command received");
            let _ = tx.send(cmd);
            ctx.request_repaint();
            "ok\n".to_string()
        }
        Err(err) => {
            tracing::warn!(line = %line.trim(), %err, "ipc rejected");
            format!("error: {err}\n")
        }
    };
    let _ = stream.write_all(response.as_bytes());
    Ok(())
}

/// Send a single command to the running wmenu instance. Used by the CLI
/// client mode in main.rs. Returns the server's reply line.
pub fn send(command: &str) -> Result<String> {
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), PORT);
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1))
        .with_context(|| format!("connecting to {addr} (is wmenu running?)"))?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    writeln!(stream, "{command}").context("writing ipc command")?;
    let mut reader = BufReader::new(stream);
    let mut reply = String::new();
    reader.read_line(&mut reply).context("reading ipc reply")?;
    Ok(reply.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_theme() {
        assert_eq!(
            IpcCommand::parse("set-theme Stone"),
            Ok(IpcCommand::SetTheme(Theme::Stone)),
        );
        assert_eq!(
            IpcCommand::parse("set-theme ink"),
            Ok(IpcCommand::SetTheme(Theme::Ink)),
        );
    }

    #[test]
    fn parses_rotate_wallpaper() {
        assert_eq!(
            IpcCommand::parse("rotate-wallpaper"),
            Ok(IpcCommand::RotateWallpaper),
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(IpcCommand::parse("").is_err());
        assert!(IpcCommand::parse("nope").is_err());
        assert!(IpcCommand::parse("set-theme").is_err());
        assert!(IpcCommand::parse("set-theme Mocha").is_err());
    }
}
