use std::fmt;

/// Keys that can be sent to the TUI application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Insert,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
    Ctrl(char),
    Alt(char),
}

/// Mouse buttons for click/scroll events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(c) => write!(f, "{c}"),
            Key::F(n) => write!(f, "F{n}"),
            Key::Ctrl(c) => write!(f, "Ctrl+{c}"),
            Key::Alt(c) => write!(f, "Alt+{c}"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// Encode a key as bytes that a terminal would send to the application.
///
/// Uses the common xterm-compatible encoding. Application cursor mode
/// is detected from the terminal parser state and affects arrow key encoding.
pub fn encode_key(key: Key, application_cursor: bool) -> Vec<u8> {
    match key {
        Key::Char(c) => c.to_string().into_bytes(),
        Key::Enter => b"\r".to_vec(),
        Key::Esc => vec![0x1b],
        Key::Tab => b"\t".to_vec(),
        Key::BackTab => b"\x1b[Z".to_vec(),
        Key::Backspace => vec![0x7f],
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::Insert => b"\x1b[2~".to_vec(),
        Key::Up => {
            if application_cursor {
                b"\x1bOA".to_vec()
            } else {
                b"\x1b[A".to_vec()
            }
        }
        Key::Down => {
            if application_cursor {
                b"\x1bOB".to_vec()
            } else {
                b"\x1b[B".to_vec()
            }
        }
        Key::Right => {
            if application_cursor {
                b"\x1bOC".to_vec()
            } else {
                b"\x1b[C".to_vec()
            }
        }
        Key::Left => {
            if application_cursor {
                b"\x1bOD".to_vec()
            } else {
                b"\x1b[D".to_vec()
            }
        }
        Key::Home => b"\x1b[H".to_vec(),
        Key::End => b"\x1b[F".to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
        Key::F(n) => encode_function_key(n),
        Key::Ctrl(c) => {
            let lower = c.to_ascii_lowercase() as u8;
            if lower.is_ascii_lowercase() {
                vec![lower - b'a' + 1]
            } else {
                match c {
                    '[' => vec![0x1b],
                    '\\' => vec![0x1c],
                    ']' => vec![0x1d],
                    '^' => vec![0x1e],
                    '_' => vec![0x1f],
                    _ => vec![c as u8],
                }
            }
        }
        Key::Alt(c) => {
            let mut out = vec![0x1b];
            out.extend(c.to_string().bytes());
            out
        }
    }
}

fn encode_function_key(n: u8) -> Vec<u8> {
    match n {
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),
        _ => Vec::new(),
    }
}

/// Encode a string as raw UTF-8 bytes for typing into the terminal.
pub fn encode_text(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// Encode text as a bracketed paste if the terminal has enabled it.
pub fn encode_paste(text: &str, bracketed_paste: bool) -> Vec<u8> {
    if bracketed_paste {
        let mut out = b"\x1b[200~".to_vec();
        out.extend_from_slice(text.as_bytes());
        out.extend_from_slice(b"\x1b[201~");
        out
    } else {
        encode_text(text)
    }
}

/// Encode an SGR 1006 mouse event.
///
/// Coordinates are 0-based cell positions; the wire protocol uses 1-based.
pub fn encode_sgr_mouse(button: MouseButton, col: u16, row: u16, release: bool) -> Vec<u8> {
    let code = match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
    };
    let suffix = if release { 'm' } else { 'M' };
    let wire_code = if release && !matches!(button, MouseButton::WheelUp | MouseButton::WheelDown) {
        3
    } else {
        code
    };
    format!(
        "\x1b[<{};{};{}{}",
        wire_code,
        col.saturating_add(1),
        row.saturating_add(1),
        suffix
    )
    .into_bytes()
}

/// Encode an SGR scroll event (wheel up/down).
pub fn encode_sgr_scroll(col: u16, row: u16, lines: i16) -> Vec<u8> {
    let code = if lines < 0 { 64 } else { 65 };
    let count = lines.unsigned_abs().max(1);
    let mut out = Vec::new();
    for _ in 0..count {
        out.extend(
            format!(
                "\x1b[<{};{};{}M",
                code,
                col.saturating_add(1),
                row.saturating_add(1),
            )
            .bytes(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_enter() {
        assert_eq!(encode_key(Key::Enter, false), b"\r");
    }

    #[test]
    fn encode_arrow_normal_mode() {
        assert_eq!(encode_key(Key::Up, false), b"\x1b[A");
        assert_eq!(encode_key(Key::Down, false), b"\x1b[B");
    }

    #[test]
    fn encode_arrow_application_mode() {
        assert_eq!(encode_key(Key::Up, true), b"\x1bOA");
        assert_eq!(encode_key(Key::Down, true), b"\x1bOB");
    }

    #[test]
    fn encode_ctrl_c() {
        assert_eq!(encode_key(Key::Ctrl('c'), false), vec![3]);
    }

    #[test]
    fn encode_alt_x() {
        assert_eq!(encode_key(Key::Alt('x'), false), b"\x1bx");
    }

    #[test]
    fn encode_f1() {
        assert_eq!(encode_key(Key::F(1), false), b"\x1bOP");
    }

    #[test]
    fn bracketed_paste() {
        let bytes = encode_paste("hello", true);
        assert!(bytes.starts_with(b"\x1b[200~"));
        assert!(bytes.ends_with(b"\x1b[201~"));
    }

    #[test]
    fn sgr_mouse_click() {
        let bytes = encode_sgr_mouse(MouseButton::Left, 5, 10, false);
        assert_eq!(bytes, b"\x1b[<0;6;11M");
    }
}
