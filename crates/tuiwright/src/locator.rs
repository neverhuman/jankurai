use serde::{Deserialize, Serialize};

use crate::screen::{Region, ScreenSnapshot};

/// A text match found on the terminal screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMatch {
    pub row: u16,
    pub col: u16,
    pub width: u16,
    pub text: String,
}

impl TextMatch {
    /// Get the center cell of this match.
    pub fn center(&self) -> (u16, u16) {
        (self.col + self.width / 2, self.row)
    }
}

/// How to select elements on the terminal screen.
#[derive(Debug, Clone)]
pub enum Selector {
    /// Match exact text substring.
    Text(String),
    /// Match a regex pattern.
    Regex(String),
    /// Select a specific cell.
    Cell(u16, u16),
    /// Select a rectangular region.
    Region(Region),
    /// Select the cursor position.
    CursorPosition,
}

/// A locator represents a way to find elements on the terminal screen.
///
/// Locators are lazy — they don't resolve until an action or assertion is performed.
/// This allows auto-waiting behavior.
pub struct Locator {
    pub(crate) selector: Selector,
}

impl Locator {
    /// Create a new locator with the given selector.
    pub fn new(selector: Selector) -> Self {
        Self { selector }
    }

    /// Resolve this locator against a screen snapshot.
    pub fn resolve(&self, screen: &ScreenSnapshot) -> Vec<TextMatch> {
        match &self.selector {
            Selector::Text(text) => screen.find_text(text),
            Selector::Regex(pattern) => screen.find_regex(pattern),
            Selector::Cell(row, col) => {
                if let Some(cell) = screen.cell(*row, *col) {
                    vec![TextMatch {
                        row: *row,
                        col: *col,
                        width: 1,
                        text: cell.text.clone(),
                    }]
                } else {
                    vec![]
                }
            }
            Selector::Region(region) => {
                let mut text = String::new();
                for r in region.row..region.row.saturating_add(region.height) {
                    for c in region.col..region.col.saturating_add(region.width) {
                        if let Some(cell) = screen.cell(r, c) {
                            if !cell.wide_continuation {
                                if cell.text.is_empty() {
                                    text.push(' ');
                                } else {
                                    text.push_str(&cell.text);
                                }
                            }
                        }
                    }
                }
                if text.trim().is_empty() {
                    vec![]
                } else {
                    vec![TextMatch {
                        row: region.row,
                        col: region.col,
                        width: region.width,
                        text,
                    }]
                }
            }
            Selector::CursorPosition => {
                vec![TextMatch {
                    row: screen.cursor_row,
                    col: screen.cursor_col,
                    width: 1,
                    text: match screen.cell(screen.cursor_row, screen.cursor_col) {
                        Some(cell) => cell.text.clone(),
                        None => String::new(),
                    },
                }]
            }
        }
    }

    /// Return only the first match.
    pub fn resolve_first(&self, screen: &ScreenSnapshot) -> Option<TextMatch> {
        self.resolve(screen).into_iter().next()
    }
}
