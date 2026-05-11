use std::fs::{self, File};
use std::path::Path;

use anyhow::{Context, Result};
use gif::{Encoder, Frame, Repeat};

use crate::render::TerminalRenderer;
use crate::screen::ScreenSnapshot;

/// Options for GIF recording output.
#[derive(Debug, Clone)]
pub struct GifOptions {
    /// Maximum frames per second.
    pub max_fps: u16,
    /// Maximum width in pixels (downscale if larger).
    pub max_width_px: Option<u32>,
    /// Whether to loop the GIF forever.
    pub loop_forever: bool,
    /// Whether to drop frames identical to the previous.
    pub drop_duplicate_frames: bool,
    /// Minimum delay between frames in centiseconds.
    pub min_delay_cs: u16,
}

impl Default for GifOptions {
    fn default() -> Self {
        Self {
            max_fps: 12,
            max_width_px: Some(1200),
            loop_forever: true,
            drop_duplicate_frames: true,
            min_delay_cs: 4,
        }
    }
}

/// A recorded frame: screen snapshot with timestamp.
#[derive(Clone)]
pub struct RecordedFrame {
    pub at_ms: u128,
    pub snapshot: ScreenSnapshot,
    pub hash: String,
}

/// Accumulates screen frames for GIF encoding.
pub struct GifRecorder {
    frames: Vec<RecordedFrame>,
    last_hash: Option<String>,
}

impl GifRecorder {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            last_hash: None,
        }
    }

    /// Capture a frame. Skips if hash matches the previous frame.
    pub fn capture_frame(&mut self, snapshot: ScreenSnapshot, at_ms: u128) {
        let hash = snapshot.stable_hash();
        if let Some(prev_hash) = &self.last_hash {
            if *prev_hash == hash {
                return; // duplicate frame
            }
        }
        self.last_hash = Some(hash.clone());
        self.frames.push(RecordedFrame {
            at_ms,
            snapshot,
            hash,
        });
    }

    /// Number of captured frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Encode all captured frames into a GIF file.
    pub fn encode_gif(
        &self,
        path: &Path,
        renderer: &TerminalRenderer,
        options: &GifOptions,
    ) -> Result<()> {
        if self.frames.is_empty() {
            anyhow::bail!("no frames to encode");
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating gif dir {}", parent.display()))?;
        }

        // Render all frames to RGBA images
        let rendered: Vec<_> = self
            .frames
            .iter()
            .map(|f| renderer.render_screen(&f.snapshot))
            .collect();

        let width = rendered[0].width() as u16;
        let height = rendered[0].height() as u16;

        let file =
            File::create(path).with_context(|| format!("creating gif file {}", path.display()))?;

        let mut encoder =
            Encoder::new(file, width, height, &[]).with_context(|| "creating GIF encoder")?;

        if options.loop_forever {
            encoder.set_repeat(Repeat::Infinite).ok();
        }

        let min_delay = options.min_delay_cs;
        let max_delay_cs = 100u16 / options.max_fps.max(1);

        for (i, img) in rendered.iter().enumerate() {
            // Calculate delay to next frame
            let delay_cs = if i + 1 < self.frames.len() {
                let delta_ms = self.frames[i + 1]
                    .at_ms
                    .saturating_sub(self.frames[i].at_ms);
                let cs = (delta_ms / 10) as u16;
                cs.clamp(min_delay, max_delay_cs.max(min_delay).max(200))
            } else {
                // Last frame: hold for 2 seconds
                200u16
            };

            // Convert RGBA to RGB for GIF
            let rgba_bytes = img.as_raw();
            let mut rgb_bytes: Vec<u8> = Vec::with_capacity(rgba_bytes.len() / 4 * 3);
            for chunk in rgba_bytes.chunks(4) {
                rgb_bytes.push(chunk[0]);
                rgb_bytes.push(chunk[1]);
                rgb_bytes.push(chunk[2]);
            }

            let mut frame = Frame::from_rgb(width, height, &rgb_bytes);
            frame.delay = delay_cs;
            encoder
                .write_frame(&frame)
                .with_context(|| format!("writing GIF frame {}", i))?;
        }

        Ok(())
    }
}

impl Default for GifRecorder {
    fn default() -> Self {
        Self::new()
    }
}
