use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{self, Stylize},
    terminal::{self, ClearType},
};
use include_dir::{include_dir, Dir};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

static POEMS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/poems");

#[derive(Parser, Debug)]
#[command(
    name = "unveilox-cli",
    version,
    about = "Unveils poems/writings in a movie roll-out style - bringing text from concealment to disclosure"
)]
struct Cli {
    /// One of: help | list | <poem_name>
    #[arg(value_name = "ACTION", default_value = "help")]
    action: String,

    /// Milliseconds per character (typewriter mode)
    #[arg(long, short, default_value_t = 25)]
    speed: u64,

    /// Use the TUI animation instead of plain typewriter
    #[arg(long)]
    tui: bool,
}

fn list_poems() {
    println!("Available writings:");
    for f in POEMS.files() {
        if let Some(stem) = f.path().file_stem().and_then(|s| s.to_str()) {
            println!("- {stem}");
        }
    }
}

fn read_poem(name: &str) -> Result<String> {
    // First try exact match with .txt
    let filename = format!("{name}.txt");
    if let Some(file) = POEMS.get_file(&filename) {
        return Ok(String::from_utf8_lossy(file.contents()).to_string());
    }

    // Fallback: match by stem
    for f in POEMS.files() {
        if let Some(stem) = f.path().file_stem().and_then(|s| s.to_str()) {
            if stem.eq_ignore_ascii_case(name) {
                return Ok(String::from_utf8_lossy(f.contents()).to_string());
            }
        }
    }

    bail!("Writing not found: {name}");
}

fn typewriter_print(text: &str, speed_ms: u64) -> Result<()> {
    let mut stdout = io::stdout();
    // Prepare screen
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Hide
    )?;
    terminal::enable_raw_mode()?;

    // Slightly stylize title if first line exists
    let mut chars = text.chars().peekable();
    let mut col: u16 = 0;
    let mut row: u16 = 0;

    while let Some(ch) = chars.next() {
        match ch {
            '\n' => {
                col = 0;
                row = row.saturating_add(1);
                execute!(stdout, cursor::MoveTo(col, row))?;
            }
            _ => {
                // Simple alternating color gag to make it a bit "silly"
                let styled = if (col as usize + row as usize) % 7 == 0 {
                    format!("{}", ch.with(style::Color::Magenta))
                } else if (col as usize) % 5 == 0 {
                    format!("{}", ch.with(style::Color::Blue))
                } else {
                    ch.to_string()
                };
                print!("{styled}");
                stdout.flush()?;
                col = col.saturating_add(1);
                // Delay between chars
                thread::sleep(Duration::from_millis(speed_ms));
            }
        }
    }

    // Finish
    stdout.flush()?;
    // Wait for a key so the text can be admired
    loop {
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(_) = event::read()? {
                break;
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    Ok(())
}

fn tui_reveal(text: &str) -> Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let total_chars = text.chars().count();
    let start = Instant::now();

    loop {
        // Increment reveal over time (about 120 chars/sec)
        let elapsed = start.elapsed().as_millis() as usize;
        let shown = (elapsed / 8).min(total_chars);

        // Build visible text safely by char count
        let visible: String = text.chars().take(shown).collect();

        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(size);

            let block = Block::default().borders(Borders::ALL).title("unveilox-cli — press q to quit");

            // Slightly "cinematic" centered title with soft color
            let paragraph = Paragraph::new(Text::from(visible))
                .block(block)
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::White));

            f.render_widget(paragraph, chunks[0]);
        })?;

        // Early exit
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }

        if shown >= total_chars {
            // After full reveal, wait for quit
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(k) = event::read()? {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => break,
                        _ => {}
                    }
                }
            }
        }
    }

    terminal.show_cursor()?;
    terminal::disable_raw_mode()?;
    // Leave alt screen
    let mut out = io::stdout();
    execute!(out, terminal::LeaveAlternateScreen)?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.action.as_str() {
        "help" => {
            // clap provides -h/--help, but print an extra hint for the positional arg
            println!("Usage: unveilox-cli [help|list|<poem_name>] [--speed N] [--tui]");
            println!("Examples:");
            println!("  unveilox-cli list");
            println!("  unveilox-cli invictus");
            println!("  unveilox-cli the_raven --tui");
            Ok(())
        }
        "list" => {
            list_poems();
            Ok(())
        }
        other => {
            let poem = read_poem(other).with_context(|| format!("while reading '{other}'"))?;
            if cli.tui {
                tui_reveal(&poem)
            } else {
                typewriter_print(&poem, cli.speed)
            }
        }
    }
}
