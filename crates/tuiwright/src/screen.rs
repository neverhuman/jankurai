use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// RGB color with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn dimmed(self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        Self {
            r: ((self.r as f32) * f) as u8,
            g: ((self.g as f32) * f) as u8,
            b: ((self.b as f32) * f) as u8,
        }
    }

    pub fn brightened(self, factor: f32) -> Self {
        let f = factor.max(1.0);
        Self {
            r: ((self.r as f32) * f).min(255.0) as u8,
            g: ((self.g as f32) * f).min(255.0) as u8,
            b: ((self.b as f32) * f).min(255.0) as u8,
        }
    }
}

/// A rectangular region in terminal cell coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub col: u16,
    pub row: u16,
    pub width: u16,
    pub height: u16,
}

impl Region {
    pub const fn new(col: u16, row: u16, width: u16, height: u16) -> Self {
        Self {
            col,
            row,
            width,
            height,
        }
    }

    pub fn center(&self) -> (u16, u16) {
        (
            self.col.saturating_add(self.width.saturating_sub(1) / 2),
            self.row.saturating_add(self.height.saturating_sub(1) / 2),
        )
    }
}

/// Snapshot of a single terminal cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSnapshot {
    pub text: String,
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub wide: bool,
    pub wide_continuation: bool,
}

/// Snapshot of the full terminal screen state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshot {
    pub rows: u16,
    pub cols: u16,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_hidden: bool,
    pub alternate_screen: bool,
    pub application_cursor: bool,
    pub bracketed_paste: bool,
    pub cells: Vec<CellSnapshot>,
}

impl ScreenSnapshot {
    /// Build a snapshot from a `vt100::Screen`.
    pub fn from_vt(screen: &vt100::Screen, theme: &crate::render::Theme) -> Self {
        let (rows, cols) = screen.size();
        let (cursor_row, cursor_col) = screen.cursor_position();
        let mut cells = Vec::with_capacity(rows as usize * cols as usize);

        for row in 0..rows {
            for col in 0..cols {
                let cell = screen.cell(row, col);
                cells.push(match cell {
                    Some(c) => {
                        let mut fg = vt_color(c.fgcolor(), theme, true);
                        let mut bg = vt_color(c.bgcolor(), theme, false);
                        if c.inverse() {
                            std::mem::swap(&mut fg, &mut bg);
                        }
                        if c.dim() {
                            fg = fg.dimmed(0.65);
                        }
                        CellSnapshot {
                            text: c.contents().to_string(),
                            fg,
                            bg,
                            bold: c.bold(),
                            dim: c.dim(),
                            italic: c.italic(),
                            underline: c.underline(),
                            inverse: c.inverse(),
                            wide: c.is_wide(),
                            wide_continuation: c.is_wide_continuation(),
                        }
                    }
                    None => CellSnapshot {
                        text: String::new(),
                        fg: theme.default_fg,
                        bg: theme.default_bg,
                        bold: false,
                        dim: false,
                        italic: false,
                        underline: false,
                        inverse: false,
                        wide: false,
                        wide_continuation: false,
                    },
                });
            }
        }

        Self {
            rows,
            cols,
            cursor_row,
            cursor_col,
            cursor_hidden: screen.hide_cursor(),
            alternate_screen: screen.alternate_screen(),
            application_cursor: screen.application_cursor(),
            bracketed_paste: screen.bracketed_paste(),
            cells,
        }
    }

    /// Get a cell at the given position.
    pub fn cell(&self, row: u16, col: u16) -> Option<&CellSnapshot> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.cells
            .get(row as usize * self.cols as usize + col as usize)
    }

    /// Get all rows as plain text strings.
    pub fn plain_rows(&self) -> Vec<String> {
        let mut rows = Vec::with_capacity(self.rows as usize);
        for row in 0..self.rows {
            let mut line = String::new();
            for col in 0..self.cols {
                let c = self.cell(row, col).expect("valid cell");
                if c.wide_continuation {
                    continue;
                }
                if c.text.is_empty() {
                    line.push(' ');
                } else {
                    line.push_str(&c.text);
                }
            }
            rows.push(line);
        }
        rows
    }

    /// Get all visible text as a single string joined by newlines.
    pub fn plain_text(&self) -> String {
        self.plain_rows().join("\n")
    }

    /// Check if the given text appears anywhere on screen.
    pub fn contains_text(&self, needle: &str) -> bool {
        self.plain_text().contains(needle)
    }

    /// Check if the given regex matches anywhere on screen.
    pub fn matches_regex(&self, pattern: &str) -> bool {
        if let Ok(re) = Regex::new(pattern) {
            let text = self.plain_text();
            re.is_match(&text)
        } else {
            false
        }
    }

    /// Find all occurrences of `needle` on screen, returning their positions.
    pub fn find_text(&self, needle: &str) -> Vec<crate::locator::TextMatch> {
        let mut matches = Vec::new();
        let rows = self.plain_rows();
        for (row_idx, line) in rows.iter().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(needle) {
                let col = start + pos;
                matches.push(crate::locator::TextMatch {
                    row: row_idx as u16,
                    col: col as u16,
                    width: needle.len() as u16,
                    text: needle.to_string(),
                });
                start = col + 1;
            }
        }
        matches
    }

    /// Find all regex matches on screen.
    pub fn find_regex(&self, pattern: &str) -> Vec<crate::locator::TextMatch> {
        let mut matches = Vec::new();
        if let Ok(re) = Regex::new(pattern) {
            let rows = self.plain_rows();
            for (row_idx, line) in rows.iter().enumerate() {
                for m in re.find_iter(line) {
                    matches.push(crate::locator::TextMatch {
                        row: row_idx as u16,
                        col: m.start() as u16,
                        width: m.len() as u16,
                        text: m.as_str().to_string(),
                    });
                }
            }
        }
        matches
    }

    /// Compute a stable SHA-256 hash of the screen content for change detection.
    pub fn stable_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.rows.to_le_bytes());
        h.update(self.cols.to_le_bytes());
        h.update(self.cursor_row.to_le_bytes());
        h.update(self.cursor_col.to_le_bytes());
        h.update([self.cursor_hidden as u8, self.alternate_screen as u8]);
        for c in &self.cells {
            h.update(c.text.as_bytes());
            h.update([0]);
            h.update([c.fg.r, c.fg.g, c.fg.b, c.bg.r, c.bg.g, c.bg.b]);
            h.update([
                c.bold as u8,
                c.dim as u8,
                c.italic as u8,
                c.underline as u8,
                c.inverse as u8,
                c.wide as u8,
                c.wide_continuation as u8,
            ]);
        }
        format!("{:x}", h.finalize())
    }
}

/// Convert a `vt100::Color` to our RGB type using the theme palette.
fn vt_color(color: vt100::Color, theme: &crate::render::Theme, foreground: bool) -> Rgb {
    match color {
        vt100::Color::Default => {
            if foreground {
                theme.default_fg
            } else {
                theme.default_bg
            }
        }
        vt100::Color::Idx(idx) => theme.color_index(idx),
        vt100::Color::Rgb(r, g, b) => Rgb::new(r, g, b),
    }
}
