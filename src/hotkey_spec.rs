//! Parse and render hotkey strings.
//!
//! Two interchangeable formats are accepted:
//!
//! - **AHK-style** — single-character modifier symbols glued in front of the
//!   key name:
//!   ```text
//!   ^  Ctrl    +  Shift    !  Alt    #  Super
//!   ```
//!   So `^A` is Ctrl+A, `^+Enter` is Ctrl+Shift+Enter, `!Space` is Alt+Space,
//!   `#1` is Super+1.
//!
//! - **Canonical** — modifier and key names separated by `+`, the format the
//!   global-hotkey crate already understands. Examples: `Ctrl+A`,
//!   `Ctrl+Shift+Enter`, `Alt+Super+Space`.
//!
//! The two formats coexist so existing `config.toml` strings keep parsing.
//! Format detection is trivial: if the first non-whitespace byte is one of
//! `^+!#`, parse AHK; otherwise parse canonical.

use std::fmt;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeySpec {
    pub mods: Modifiers,
    pub key: Code,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    NoKey,
    UnknownKey(String),
    DuplicateKey,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty"),
            ParseError::NoKey => write!(f, "no key after modifiers"),
            ParseError::UnknownKey(k) => write!(f, "unknown key '{k}'"),
            ParseError::DuplicateKey => write!(f, "more than one main key"),
        }
    }
}

impl std::error::Error for ParseError {}

impl HotkeySpec {
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::Empty);
        }
        if starts_with_ahk_symbol(s) {
            parse_ahk(s)
        } else {
            parse_canonical(s)
        }
    }

    /// Compact representation: `^+Enter`, `!Space`, `#A`, etc.
    pub fn to_ahk(self) -> String {
        let mut out = String::new();
        // Canonical AHK ordering: ^ + ! #
        if self.mods.contains(Modifiers::CONTROL) {
            out.push('^');
        }
        if self.mods.contains(Modifiers::SHIFT) {
            out.push('+');
        }
        if self.mods.contains(Modifiers::ALT) {
            out.push('!');
        }
        if self.mods.contains(Modifiers::SUPER) {
            out.push('#');
        }
        out.push_str(key_canonical_name(self.key));
        out
    }

    /// Expanded form for previews: `Ctrl + Shift + Enter`.
    pub fn to_human(self) -> String {
        let mut parts: Vec<&'static str> = Vec::new();
        if self.mods.contains(Modifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.mods.contains(Modifiers::SHIFT) {
            parts.push("Shift");
        }
        if self.mods.contains(Modifiers::ALT) {
            parts.push("Alt");
        }
        if self.mods.contains(Modifiers::SUPER) {
            parts.push("Super");
        }
        parts.push(key_canonical_name(self.key));
        parts.join(" + ")
    }

    /// Build the `global_hotkey` registration value for this spec.
    pub fn to_global_hotkey(self) -> HotKey {
        HotKey::new(Some(self.mods), self.key)
    }
}

fn starts_with_ahk_symbol(s: &str) -> bool {
    matches!(s.chars().next(), Some('^' | '+' | '!' | '#'))
}

fn parse_ahk(s: &str) -> Result<HotkeySpec, ParseError> {
    let mut mods = Modifiers::empty();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '^' => {
                mods |= Modifiers::CONTROL;
                chars.next();
            }
            '+' => {
                mods |= Modifiers::SHIFT;
                chars.next();
            }
            '!' => {
                mods |= Modifiers::ALT;
                chars.next();
            }
            '#' => {
                mods |= Modifiers::SUPER;
                chars.next();
            }
            _ => break,
        }
    }
    let rest: String = chars.collect();
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(ParseError::NoKey);
    }
    let key = parse_key_name(rest).ok_or_else(|| ParseError::UnknownKey(rest.to_string()))?;
    Ok(HotkeySpec { mods, key })
}

fn parse_canonical(s: &str) -> Result<HotkeySpec, ParseError> {
    let tokens: Vec<&str> = s
        .split('+')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;
    for tok in tokens {
        let upper = tok.to_uppercase();
        match upper.as_str() {
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "SHIFT" => mods |= Modifiers::SHIFT,
            "ALT" | "OPTION" => mods |= Modifiers::ALT,
            "SUPER" | "WIN" | "META" | "CMD" | "COMMAND" => mods |= Modifiers::SUPER,
            "CMDORCTRL" | "COMMANDORCONTROL" | "CMDORCONTROL" | "COMMANDORCTRL" => {
                #[cfg(target_os = "macos")]
                {
                    mods |= Modifiers::SUPER;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    mods |= Modifiers::CONTROL;
                }
            }
            _ => {
                if key.is_some() {
                    return Err(ParseError::DuplicateKey);
                }
                key = Some(
                    parse_key_name(tok).ok_or_else(|| ParseError::UnknownKey(tok.to_string()))?,
                );
            }
        }
    }
    Ok(HotkeySpec {
        mods,
        key: key.ok_or(ParseError::NoKey)?,
    })
}

fn parse_key_name(s: &str) -> Option<Code> {
    use Code::*;
    let upper = s.to_uppercase();
    Some(match upper.as_str() {
        "A" => KeyA,
        "B" => KeyB,
        "C" => KeyC,
        "D" => KeyD,
        "E" => KeyE,
        "F" => KeyF,
        "G" => KeyG,
        "H" => KeyH,
        "I" => KeyI,
        "J" => KeyJ,
        "K" => KeyK,
        "L" => KeyL,
        "M" => KeyM,
        "N" => KeyN,
        "O" => KeyO,
        "P" => KeyP,
        "Q" => KeyQ,
        "R" => KeyR,
        "S" => KeyS,
        "T" => KeyT,
        "U" => KeyU,
        "V" => KeyV,
        "W" => KeyW,
        "X" => KeyX,
        "Y" => KeyY,
        "Z" => KeyZ,
        "0" => Digit0,
        "1" => Digit1,
        "2" => Digit2,
        "3" => Digit3,
        "4" => Digit4,
        "5" => Digit5,
        "6" => Digit6,
        "7" => Digit7,
        "8" => Digit8,
        "9" => Digit9,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "F13" => F13,
        "F14" => F14,
        "F15" => F15,
        "F16" => F16,
        "F17" => F17,
        "F18" => F18,
        "F19" => F19,
        "F20" => F20,
        "F21" => F21,
        "F22" => F22,
        "F23" => F23,
        "F24" => F24,
        "ENTER" | "RETURN" => Enter,
        "TAB" => Tab,
        "SPACE" => Space,
        "ESC" | "ESCAPE" => Escape,
        "BACKSPACE" | "BS" => Backspace,
        "DELETE" | "DEL" => Delete,
        "INSERT" | "INS" => Insert,
        "HOME" => Home,
        "END" => End,
        "PAGEUP" | "PGUP" => PageUp,
        "PAGEDOWN" | "PGDN" => PageDown,
        "UP" | "ARROWUP" => ArrowUp,
        "DOWN" | "ARROWDOWN" => ArrowDown,
        "LEFT" | "ARROWLEFT" => ArrowLeft,
        "RIGHT" | "ARROWRIGHT" => ArrowRight,
        "-" | "MINUS" => Minus,
        "=" | "EQUAL" => Equal,
        "," | "COMMA" => Comma,
        "." | "PERIOD" => Period,
        "/" | "SLASH" => Slash,
        ";" | "SEMICOLON" => Semicolon,
        "'" | "QUOTE" => Quote,
        "`" | "BACKQUOTE" => Backquote,
        "\\" | "BACKSLASH" => Backslash,
        "[" | "BRACKETLEFT" => BracketLeft,
        "]" | "BRACKETRIGHT" => BracketRight,
        "CAPSLOCK" => CapsLock,
        "PRINTSCREEN" | "PRTSC" => PrintScreen,
        "SCROLLLOCK" => ScrollLock,
        "PAUSE" | "PAUSEBREAK" => Pause,
        _ => return None,
    })
}

fn key_canonical_name(code: Code) -> &'static str {
    use Code::*;
    match code {
        KeyA => "A",
        KeyB => "B",
        KeyC => "C",
        KeyD => "D",
        KeyE => "E",
        KeyF => "F",
        KeyG => "G",
        KeyH => "H",
        KeyI => "I",
        KeyJ => "J",
        KeyK => "K",
        KeyL => "L",
        KeyM => "M",
        KeyN => "N",
        KeyO => "O",
        KeyP => "P",
        KeyQ => "Q",
        KeyR => "R",
        KeyS => "S",
        KeyT => "T",
        KeyU => "U",
        KeyV => "V",
        KeyW => "W",
        KeyX => "X",
        KeyY => "Y",
        KeyZ => "Z",
        Digit0 => "0",
        Digit1 => "1",
        Digit2 => "2",
        Digit3 => "3",
        Digit4 => "4",
        Digit5 => "5",
        Digit6 => "6",
        Digit7 => "7",
        Digit8 => "8",
        Digit9 => "9",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        F13 => "F13",
        F14 => "F14",
        F15 => "F15",
        F16 => "F16",
        F17 => "F17",
        F18 => "F18",
        F19 => "F19",
        F20 => "F20",
        F21 => "F21",
        F22 => "F22",
        F23 => "F23",
        F24 => "F24",
        Enter => "Enter",
        Tab => "Tab",
        Space => "Space",
        Escape => "Esc",
        Backspace => "Backspace",
        Delete => "Delete",
        Insert => "Insert",
        Home => "Home",
        End => "End",
        PageUp => "PageUp",
        PageDown => "PageDown",
        ArrowUp => "Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        Minus => "-",
        Equal => "=",
        Comma => ",",
        Period => ".",
        Slash => "/",
        Semicolon => ";",
        Quote => "'",
        Backquote => "`",
        Backslash => "\\",
        BracketLeft => "[",
        BracketRight => "]",
        CapsLock => "CapsLock",
        PrintScreen => "PrintScreen",
        ScrollLock => "ScrollLock",
        Pause => "Pause",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ahk_single_modifier() {
        let s = HotkeySpec::parse("^A").unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL);
        assert_eq!(s.key, Code::KeyA);
    }

    #[test]
    fn ahk_combo() {
        let s = HotkeySpec::parse("^+!Enter").unwrap();
        assert_eq!(
            s.mods,
            Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT
        );
        assert_eq!(s.key, Code::Enter);
    }

    #[test]
    fn ahk_super_digit() {
        let s = HotkeySpec::parse("#1").unwrap();
        assert_eq!(s.mods, Modifiers::SUPER);
        assert_eq!(s.key, Code::Digit1);
    }

    #[test]
    fn canonical_existing_config() {
        let s = HotkeySpec::parse("Alt+Super+Space").unwrap();
        assert_eq!(s.mods, Modifiers::ALT | Modifiers::SUPER);
        assert_eq!(s.key, Code::Space);
    }

    #[test]
    fn canonical_case_insensitive() {
        let s = HotkeySpec::parse("ctrl+shift+enter").unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(s.key, Code::Enter);
    }

    #[test]
    fn bare_key() {
        let s = HotkeySpec::parse("Escape").unwrap();
        assert_eq!(s.mods, Modifiers::empty());
        assert_eq!(s.key, Code::Escape);
    }

    #[test]
    fn unknown_key_yields_unknown_error() {
        let err = HotkeySpec::parse("^Retrn").unwrap_err();
        assert_eq!(err, ParseError::UnknownKey("Retrn".into()));
    }

    #[test]
    fn modifiers_without_key() {
        let err = HotkeySpec::parse("^+").unwrap_err();
        assert_eq!(err, ParseError::NoKey);
    }

    #[test]
    fn empty_input() {
        let err = HotkeySpec::parse("   ").unwrap_err();
        assert_eq!(err, ParseError::Empty);
    }

    #[test]
    fn canonical_two_main_keys() {
        let err = HotkeySpec::parse("A+B").unwrap_err();
        assert_eq!(err, ParseError::DuplicateKey);
    }

    #[test]
    fn round_trip_ahk_form() {
        let s = HotkeySpec::parse("Ctrl+Shift+A").unwrap();
        assert_eq!(s.to_ahk(), "^+A");
    }

    #[test]
    fn round_trip_human_form() {
        let s = HotkeySpec::parse("^+!Enter").unwrap();
        assert_eq!(s.to_human(), "Ctrl + Shift + Alt + Enter");
    }

    #[test]
    fn parsing_is_idempotent_through_ahk() {
        let original = HotkeySpec::parse("Ctrl+Shift+Enter").unwrap();
        let round = HotkeySpec::parse(&original.to_ahk()).unwrap();
        assert_eq!(original, round);
    }
}
