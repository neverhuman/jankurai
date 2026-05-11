use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tuiwright::{GifOptions, Page, SpawnConfig};

#[derive(Parser)]
#[command(name = "tuiwright", about = "Playwright-style TUI testing CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Capture a PNG screenshot of a TUI application.
    Screenshot {
        /// Terminal columns.
        #[arg(long, default_value_t = 80)]
        cols: u16,
        /// Terminal rows.
        #[arg(long, default_value_t = 24)]
        rows: u16,
        /// Text to wait for before capturing.
        #[arg(long)]
        wait_text: Option<String>,
        /// Wait timeout in seconds.
        #[arg(long, default_value_t = 5)]
        wait_timeout: u64,
        /// Output PNG path.
        #[arg(long, short)]
        out: String,
        /// Command and arguments to run (after --).
        #[arg(last = true, required = true)]
        cmd: Vec<String>,
    },
    /// Record a GIF of a TUI application.
    Record {
        /// Terminal columns.
        #[arg(long, default_value_t = 80)]
        cols: u16,
        /// Terminal rows.
        #[arg(long, default_value_t = 24)]
        rows: u16,
        /// Text to wait for before starting recording.
        #[arg(long)]
        wait_text: Option<String>,
        /// Recording duration in seconds.
        #[arg(long, default_value_t = 3)]
        seconds: u64,
        /// Output GIF path.
        #[arg(long, short)]
        out: String,
        /// Command and arguments to run (after --).
        #[arg(last = true, required = true)]
        cmd: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Screenshot {
            cols,
            rows,
            wait_text,
            wait_timeout,
            out,
            cmd,
        } => {
            let (program, args) = split_cmd(&cmd)?;
            let page = Page::spawn(SpawnConfig::new(program).args(args).size(cols, rows))?;

            if let Some(text) = wait_text {
                page.wait_for_text(&text, Duration::from_secs(wait_timeout))?;
            } else {
                page.wait_until_idle(Duration::from_millis(500))?;
            }

            page.screenshot(&out)?;
            eprintln!("Screenshot saved to {out}");
            page.kill()?;
        }
        Commands::Record {
            cols,
            rows,
            wait_text,
            seconds,
            out,
            cmd,
        } => {
            let (program, args) = split_cmd(&cmd)?;
            let page = Page::spawn(
                SpawnConfig::new(program)
                    .args(args)
                    .size(cols, rows)
                    .record(true),
            )?;

            if let Some(text) = wait_text {
                page.wait_for_text(&text, Duration::from_secs(5))?;
            } else {
                page.wait_until_idle(Duration::from_millis(500))?;
            }

            page.start_recording()?;
            std::thread::sleep(Duration::from_secs(seconds));
            page.stop_recording_gif(&out, GifOptions::default())?;
            eprintln!("GIF saved to {out}");
            page.kill()?;
        }
    }

    Ok(())
}

fn split_cmd(cmd: &[String]) -> Result<(&str, Vec<&str>)> {
    if cmd.is_empty() {
        anyhow::bail!("command cannot be empty");
    }
    let program = cmd[0].as_str();
    let args: Vec<&str> = cmd[1..].iter().map(|s| s.as_str()).collect();
    Ok((program, args))
}
