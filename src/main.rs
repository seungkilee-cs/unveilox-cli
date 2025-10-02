use std::io::{self, Write};
use std::str::FromStr;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
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

const DEFAULT_SPEED: u64 = 25;
const MIN_SPEED: u64 = 1;
const MAX_SPEED: u64 = 1_000;

#[derive(Debug, Clone)]
enum Action {
    Help,
    List,
    Show(String),
}

impl FromStr for Action {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("Action must not be empty".to_string());
        }

        if trimmed.eq_ignore_ascii_case("help") {
            Ok(Action::Help)
        } else if trimmed.eq_ignore_ascii_case("list") {
            Ok(Action::List)
        } else {
            Ok(Action::Show(trimmed.to_string()))
        }
    }
}

fn parse_action(raw: &str) -> std::result::Result<Action, String> {
    Action::from_str(raw)
}

fn parse_speed(raw: &str) -> std::result::Result<u64, String> {
    let speed: u64 = raw
        .parse()
        .map_err(|_| format!("`{raw}` is not a valid positive integer"))?;

    if !(MIN_SPEED..=MAX_SPEED).contains(&speed) {
        Err(format!(
            "speed must be between {MIN_SPEED} and {MAX_SPEED} milliseconds"
        ))
    } else {
        Ok(speed)
    }
}

struct TerminalGuard {
    raw_mode: bool,
    alt_screen: bool,
    cursor_hidden: bool,
}

impl TerminalGuard {
    fn enter(hide_cursor: bool) -> Result<Self> {
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;

        if hide_cursor {
            execute!(stdout, cursor::Hide)?;
        }

        Ok(Self {
            raw_mode: true,
            alt_screen: true,
            cursor_hidden: hide_cursor,
        })
    }

    fn clear(&self) -> Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<()> {
        if self.cursor_hidden {
            let mut stdout = io::stdout();
            execute!(stdout, cursor::Show)?;
            self.cursor_hidden = false;
        }
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> Result<()> {
        if self.raw_mode {
            terminal::disable_raw_mode()?;
            self.raw_mode = false;
        }
        Ok(())
    }

    fn leave_alt_screen(&mut self) -> Result<()> {
        if self.alt_screen {
            let mut stdout = io::stdout();
            execute!(stdout, terminal::LeaveAlternateScreen)?;
            self.alt_screen = false;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.show_cursor()?;
        self.disable_raw_mode()?;
        self.leave_alt_screen()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.show_cursor();
        let _ = self.disable_raw_mode();
        let _ = self.leave_alt_screen();
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "unveilox-cli",
    version,
    about = "Unveils poems/writings in a movie roll-out style - bringing text from concealment to disclosure"
)]
struct Cli {
    /// One of: help | list | <poem_name>
    #[arg(value_name = "ACTION", value_parser = parse_action, default_value = "help")]
    action: Action,

    /// Milliseconds per character (typewriter mode)
    #[arg(long, short, default_value_t = DEFAULT_SPEED, value_parser = parse_speed)]
    speed: u64,

    /// Use the TUI animation instead of plain typewriter
    #[arg(long)]
    tui: bool,
}

fn list_poems() {
    let mut names: Vec<_> = POEMS
        .files()
        .filter_map(|f| {
            f.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
        })
        .collect();

    names.sort_unstable();

    if names.is_empty() {
        println!("No writings bundled. Add files under assets/poems/.");
        return;
    }

    println!("Available writings:");
    for name in names {
        println!("- {name}");
    }
}

fn read_poem(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("Writing name must not be empty");
    }

    // First try exact match with .txt
    let filename = format!("{trimmed}.txt");
    if let Some(file) = POEMS.get_file(&filename) {
        return Ok(String::from_utf8_lossy(file.contents()).into_owned());
    }

    if let Some(file) = POEMS.files().find(|f| {
        f.path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.eq_ignore_ascii_case(trimmed))
            .unwrap_or(false)
    }) {
        return Ok(String::from_utf8_lossy(file.contents()).into_owned());
    }

    bail!("Writing not found: {trimmed}");
}

fn typewriter_print(text: &str, speed_ms: u64) -> Result<()> {
    let mut guard = TerminalGuard::enter(true)?;
    guard.clear()?;

    let mut stdout = io::stdout();
    let mut col: u16 = 0;
    let mut row: u16 = 0;
    let mut exit_requested = false;

    for ch in text.chars() {
        match ch {
            '\n' => {
                col = 0;
                row = row.saturating_add(1);
                execute!(&mut stdout, cursor::MoveTo(col, row))?;
            }
            _ => {
                let styled = if (col as usize + row as usize) % 7 == 0 {
                    format!("{}", ch.with(style::Color::Magenta))
                } else if (col as usize) % 5 == 0 {
                    format!("{}", ch.with(style::Color::Blue))
                } else {
                    ch.to_string()
                };
                write!(&mut stdout, "{styled}")?;
                stdout.flush()?;
                col = col.saturating_add(1);
            }
        }

        if poll_for_exit(Duration::from_millis(speed_ms))? {
            exit_requested = true;
            break;
        }
    }

    stdout.flush()?;

    if !exit_requested {
        while !poll_for_exit(Duration::from_millis(100))? {}
    }

    guard.finish()?;
    Ok(())
}

fn tui_reveal(text: &str) -> Result<()> {
    let mut guard = TerminalGuard::enter(true)?;

    let backend = CrosstermBackend::new(io::stdout());
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

            let block = Block::default()
                .borders(Borders::ALL)
                .title("unveilox-cli — press q to quit");

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
            match event::read()? {
                Event::Key(k) if is_exit_key(&k) => break,
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if shown >= total_chars {
            // After full reveal, wait for quit
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(k) if is_exit_key(&k) => break,
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }
    }

    terminal.show_cursor()?;
    guard.finish()?;
    Ok(())
}

fn main() -> Result<()> {
    let Cli { action, speed, tui } = Cli::parse();

    match action {
        Action::Help => {
            println!("Usage: unveilox-cli [help|list|<poem_name>] [--speed N] [--tui]");
            println!("Examples:");
            println!("  unveilox-cli list");
            println!("  unveilox-cli invictus");
            println!("  unveilox-cli the_raven --tui");
            Ok(())
        }
        Action::List => {
            list_poems();
            Ok(())
        }
        Action::Show(name) => {
            let poem = read_poem(&name).with_context(|| format!("while reading '{name}'"))?;
            if tui {
                tui_reveal(&poem)
            } else {
                typewriter_print(&poem, speed)
            }
        }
    }
}

fn is_exit_key(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        _ => false,
    }
}

fn poll_for_exit(timeout: Duration) -> Result<bool> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            return Ok(is_exit_key(&key));
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_from_str_parses_variants() {
        assert!(matches!(Action::from_str("help").unwrap(), Action::Help));
        assert!(matches!(Action::from_str("LIST").unwrap(), Action::List));
        match Action::from_str("Invictus").unwrap() {
            Action::Show(name) => assert_eq!(name, "Invictus"),
            _ => panic!("expected show variant"),
        }
    }

    #[test]
    fn parse_speed_enforces_bounds() {
        assert_eq!(parse_speed("25").unwrap(), 25);
        assert!(parse_speed("0").is_err());
        assert!(parse_speed("1001").is_err());
        assert!(parse_speed("not-a-number").is_err());
    }

    #[test]
    fn poem_lookup_is_case_insensitive() {
        let lower = read_poem("invictus").expect("poem should load");
        let upper = read_poem("INVICtus").expect("poem should load");
        assert_eq!(lower, upper);
    }

    #[test]
    fn empty_poem_name_is_rejected() {
        let err = read_poem("   ").expect_err("empty name must fail");
        assert!(err.to_string().contains("must not be empty"));
    }
}
