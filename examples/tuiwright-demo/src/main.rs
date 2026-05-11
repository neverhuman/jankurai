use std::io::{self, stdout, Write};
use std::time::Duration;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Color, Print, SetBackgroundColor, SetForegroundColor, Stylize};
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use crossterm::{execute, queue};

fn main() -> io::Result<()> {
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        Hide,
        SetTitle("Tuiwright Demo Counter"),
    )?;

    let mut counter: i64 = 0;
    let mut running = true;

    while running {
        draw(&mut stdout, counter)?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }) => {
                    running = false;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up, ..
                }) => {
                    counter += 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    ..
                }) => {
                    counter = counter.saturating_sub(1);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('r'),
                    ..
                }) => {
                    counter = 0;
                }
                Event::Resize(..) => {
                    // Redraw on resize
                }
                _ => {}
            }
        }
    }

    execute!(stdout, Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn draw(stdout: &mut impl Write, counter: i64) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;

    queue!(stdout, Clear(ClearType::All))?;

    // Title bar
    queue!(
        stdout,
        MoveTo(0, 0),
        SetBackgroundColor(Color::DarkBlue),
        SetForegroundColor(Color::White),
    )?;
    let title = " Tuiwright Demo Counter ";
    let padding = " ".repeat((cols as usize).saturating_sub(title.len()));
    queue!(stdout, Print(format!("{title}{padding}")))?;

    // Counter display
    let counter_text = format!("Counter: {counter}");
    let cx = cols.saturating_sub(counter_text.len() as u16) / 2;
    let cy = rows / 2;
    queue!(
        stdout,
        MoveTo(cx, cy),
        SetBackgroundColor(Color::Reset),
        SetForegroundColor(Color::Green),
        Print(counter_text.bold()),
    )?;

    // Help text
    let help = "Up/Down change | r reset | q quit";
    let hx = cols.saturating_sub(help.len() as u16) / 2;
    queue!(
        stdout,
        MoveTo(hx, cy + 2),
        SetForegroundColor(Color::DarkGrey),
        Print(help),
    )?;

    // Status line
    queue!(
        stdout,
        MoveTo(0, rows - 1),
        SetBackgroundColor(Color::DarkGrey),
        SetForegroundColor(Color::White),
    )?;
    let status = format!(" {}x{} ", cols, rows);
    let status_pad = " ".repeat((cols as usize).saturating_sub(status.len()));
    queue!(stdout, Print(format!("{status}{status_pad}")))?;

    stdout.flush()?;
    Ok(())
}
