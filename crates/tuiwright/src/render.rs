use std::cmp::max;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use rusttype::{point, Font, Scale};

use crate::screen::{Rgb, ScreenSnapshot};

/// Rendering options for terminal screenshots.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Cell width in pixels.
    pub cell_width: u32,
    /// Cell height in pixels.
    pub cell_height: u32,
    /// Whether to draw the cursor.
    pub draw_cursor: bool,
    /// Padding around the terminal in pixels.
    pub padding: u32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            cell_width: 10,
            cell_height: 20,
            draw_cursor: true,
            padding: 8,
        }
    }
}

impl RenderOptions {
    /// Deterministic CI options — consistent across environments.
    pub fn ci() -> Self {
        Self {
            cell_width: 10,
            cell_height: 20,
            draw_cursor: true,
            padding: 8,
        }
    }
}

/// Terminal color theme with 256-color ANSI palette.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub default_fg: Rgb,
    pub default_bg: Rgb,
    pub cursor_color: Rgb,
    /// 256-entry ANSI/xterm palette.
    pub ansi: Vec<Rgb>,
}

impl Theme {
    /// Standard xterm dark theme.
    pub fn xterm_dark() -> Self {
        let mut ansi = vec![Rgb::new(0, 0, 0); 256];
        let base = [
            (0x00, 0x00, 0x00),
            (0xcd, 0x00, 0x00),
            (0x00, 0xcd, 0x00),
            (0xcd, 0xcd, 0x00),
            (0x00, 0x00, 0xee),
            (0xcd, 0x00, 0xcd),
            (0x00, 0xcd, 0xcd),
            (0xe5, 0xe5, 0xe5),
            (0x7f, 0x7f, 0x7f),
            (0xff, 0x00, 0x00),
            (0x00, 0xff, 0x00),
            (0xff, 0xff, 0x00),
            (0x5c, 0x5c, 0xff),
            (0xff, 0x00, 0xff),
            (0x00, 0xff, 0xff),
            (0xff, 0xff, 0xff),
        ];
        for (i, (r, g, b)) in base.into_iter().enumerate() {
            ansi[i] = Rgb::new(r, g, b);
        }
        // 6x6x6 color cube (indices 16–231)
        let levels = [0u8, 95, 135, 175, 215, 255];
        let mut idx = 16;
        for r in levels {
            for g in levels {
                for b in levels {
                    ansi[idx] = Rgb::new(r, g, b);
                    idx += 1;
                }
            }
        }
        // Grayscale ramp (indices 232–255)
        for i in 0..24 {
            let v = (8 + i * 10) as u8;
            ansi[232 + i] = Rgb::new(v, v, v);
        }

        Self {
            name: "xterm-dark".to_string(),
            default_fg: Rgb::new(0xd0, 0xd0, 0xd0),
            default_bg: Rgb::new(0x1e, 0x1e, 0x2e),
            cursor_color: Rgb::new(0xff, 0xff, 0xff),
            ansi,
        }
    }

    /// Look up a color index in the 256-color palette.
    pub fn color_index(&self, idx: u8) -> Rgb {
        self.ansi
            .get(idx as usize)
            .copied()
            .unwrap_or(self.default_fg)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::xterm_dark()
    }
}

/// Renders terminal screen snapshots to pixel images.
pub struct TerminalRenderer {
    pub options: RenderOptions,
    pub theme: Theme,
}

impl TerminalRenderer {
    pub fn new(options: RenderOptions, theme: Theme) -> Self {
        Self { options, theme }
    }

    /// Render a screen snapshot to an RGBA image.
    pub fn render_screen(&self, screen: &ScreenSnapshot) -> RgbaImage {
        let padding = self.options.padding;
        let width = screen.cols as u32 * self.options.cell_width + 2 * padding;
        let height = screen.rows as u32 * self.options.cell_height + 2 * padding;
        let bg = rgb_to_rgba(self.theme.default_bg);
        let mut img = RgbaImage::from_pixel(width.max(1), height.max(1), bg);

        for row in 0..screen.rows {
            for col in 0..screen.cols {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };
                if cell.wide_continuation {
                    continue;
                }

                let mut fg = cell.fg;
                let bg_color = cell.bg;

                if cell.bold {
                    fg = fg.brightened(1.15);
                }

                let x = padding + col as u32 * self.options.cell_width;
                let y = padding + row as u32 * self.options.cell_height;

                // Draw cell background
                fill_rect(
                    &mut img,
                    x,
                    y,
                    self.options.cell_width,
                    self.options.cell_height,
                    rgb_to_rgba(bg_color),
                );

                // Draw cell text using rusttype and embedded TTF
                if !cell.text.is_empty() && cell.text != " " {
                    let ch = cell.text.chars().next().unwrap_or(' ');
                    draw_char_ttf(
                        &mut img,
                        x,
                        y,
                        self.options.cell_width,
                        self.options.cell_height,
                        ch,
                        rgb_to_rgba(fg),
                    );
                }

                // Draw underline
                if cell.underline {
                    let thickness = max(1, self.options.cell_height / 16);
                    fill_rect(
                        &mut img,
                        x,
                        y + self.options.cell_height.saturating_sub(thickness + 1),
                        self.options.cell_width,
                        thickness,
                        rgb_to_rgba(fg),
                    );
                }
            }
        }

        // Draw cursor
        if self.options.draw_cursor && !screen.cursor_hidden {
            let crow = screen.cursor_row;
            let ccol = screen.cursor_col;
            if crow < screen.rows && ccol < screen.cols {
                let x = padding + ccol as u32 * self.options.cell_width;
                let y = padding + crow as u32 * self.options.cell_height;
                let thickness = max(2, self.options.cell_height / 8);
                fill_rect(
                    &mut img,
                    x,
                    y + self.options.cell_height.saturating_sub(thickness),
                    self.options.cell_width,
                    thickness,
                    rgb_to_rgba(self.theme.cursor_color),
                );
            }
        }

        img
    }

    /// Render and save a screenshot to the given path as PNG.
    pub fn save_screenshot(&self, screen: &ScreenSnapshot, path: &Path) -> Result<()> {
        let img = self.render_screen(screen);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating screenshot dir {}", parent.display()))?;
        }
        img.save(path)
            .with_context(|| format!("saving screenshot to {}", path.display()))?;
        Ok(())
    }

    /// Image dimensions for the given screen size.
    pub fn image_size(&self, cols: u16, rows: u16) -> (u32, u32) {
        let w = cols as u32 * self.options.cell_width + 2 * self.options.padding;
        let h = rows as u32 * self.options.cell_height + 2 * self.options.padding;
        (w, h)
    }
}

fn rgb_to_rgba(c: Rgb) -> Rgba<u8> {
    Rgba([c.r, c.g, c.b, 255])
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, color);
            }
        }
    }
}

const FONT_DATA: &[u8] = include_bytes!("../assets/JetBrainsMono-Regular.ttf");

/// Draw a character using rusttype, anti-aliased and scaled.
fn draw_char_ttf(
    img: &mut RgbaImage,
    x: u32,
    y: u32,
    cell_w: u32,
    cell_h: u32,
    ch: char,
    color: Rgba<u8>,
) {
    let font = Font::try_from_bytes(FONT_DATA).expect("Failed to load font");

    // Calculate scaling to perfectly fit the monospace cell
    // Get advance width of 'M' to determine required width stretch
    let uniform_scale = Scale::uniform(cell_h as f32);
    let m_metrics = font.glyph('M').scaled(uniform_scale).h_metrics();
    // Stretch x so that advance_width == cell_w
    let width_stretch = cell_w as f32 / m_metrics.advance_width;
    let final_scale = Scale {
        x: cell_h as f32 * width_stretch,
        y: cell_h as f32,
    };

    let glyph = font.glyph(ch);
    if glyph.id().0 == 0 {
        // Unknown glyph, could draw a box but we'll let it be for now
    }

    let scaled = glyph.scaled(final_scale);
    let v_metrics = font.v_metrics(final_scale);
    let h_metrics = scaled.h_metrics();

    // Center it within the cell width
    let offset_x = (cell_w as f32 - h_metrics.advance_width) / 2.0;

    // Position the glyph. Ascent is the distance from the baseline to the top.
    let p = point(x as f32 + offset_x, y as f32 + v_metrics.ascent);
    let positioned = scaled.positioned(p);

    if let Some(bb) = positioned.pixel_bounding_box() {
        positioned.draw(|gx, gy, v| {
            let px = bb.min.x + gx as i32;
            let py = bb.min.y + gy as i32;
            // Clamp rendering within the cell boundaries to prevent bleed
            if px >= x as i32
                && px < (x + cell_w) as i32
                && py >= y as i32
                && py < (y + cell_h) as i32
                && px >= 0
                && px < img.width() as i32
                && py >= 0
                && py < img.height() as i32
            {
                let px = px as u32;
                let py = py as u32;
                let mut pixel = *img.get_pixel(px, py);

                // Alpha blend
                pixel[0] = ((1.0 - v) * pixel[0] as f32 + v * color[0] as f32) as u8;
                pixel[1] = ((1.0 - v) * pixel[1] as f32 + v * color[1] as f32) as u8;
                pixel[2] = ((1.0 - v) * pixel[2] as f32 + v * color[2] as f32) as u8;

                img.put_pixel(px, py, pixel);
            }
        });
    }
}
