//! A tool for exporting release content.
//!
//! This terminal-based tool generates a release content file
//! from the content of the `release-content` directory.
//!
//! To run this tool, use the following command from the `bevy` repository root:
//!
//! ```sh
//! cargo run -p export-content
//! ```

use std::{
    io,
    panic::{set_hook, take_hook},
};

use app::App;
use miette::{IntoDiagnostic, Result};
use ratatui::{
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::*,
};

use crate::app::Content;

mod app;

fn main() -> Result<()> {
    let check = std::env::args().any(|arg| arg == "--check");
    if check {
        Content::load().unwrap();
        return Ok(());
    }

    init_panic_hook();
    let mut terminal = init_terminal().unwrap();
    let res = run_app(&mut terminal);
    restore_terminal().unwrap();
    res
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let app = App::new()?;
    app.run(terminal)
}

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

fn init_terminal() -> Result<Terminal<impl Backend>> {
    enable_raw_mode().into_diagnostic()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).into_diagnostic()?;
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend).into_diagnostic()?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode().into_diagnostic()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).into_diagnostic()?;
    Ok(())
}
