use std::{env, fs, io::Write, path::PathBuf};

use miette::{diagnostic, Context, Diagnostic, IntoDiagnostic, NamedSource, Result};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    prelude::*,
    widgets::*,
};
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

enum Mode {
    ReleaseNotes,
    MigrationGuides,
}

pub struct App {
    content_dir: PathBuf,
    release_notes: Vec<Entry>,
    release_notes_state: ListState,
    migration_guides: Vec<Entry>,
    migration_guide_state: ListState,
    text_entry: Option<String>,
    mode: Mode,
    exit: bool,
}

pub struct Content {
    content_dir: PathBuf,
    migration_guides: Vec<Entry>,
    release_notes: Vec<Entry>,
}

impl Content {
    pub fn load() -> Result<Self> {
        let exe_dir = env::current_exe()
            .into_diagnostic()
            .wrap_err("failed to determine path to binary")?;

        let content_dir = exe_dir
            .ancestors()
            .nth(3)
            .ok_or(diagnostic!("failed to determine path to repo root"))?
            .join("release-content");

        let release_notes_dir = content_dir.join("release-notes");
        let release_notes = load_content(release_notes_dir, "release note")?;

        let migration_guides_dir = content_dir.join("migration-guides");
        let migration_guides = load_content(migration_guides_dir, "migration guide")?;
        Ok(Content {
            content_dir,
            migration_guides,
            release_notes,
        })
    }
}

impl App {
    pub fn new() -> Result<App> {
        let Content {
            content_dir,
            release_notes,
            migration_guides,
        } = Content::load()?;

        Ok(App {
            content_dir,
            release_notes,
            release_notes_state: ListState::default().with_selected(Some(0)),
            migration_guides,
            migration_guide_state: ListState::default().with_selected(Some(0)),
            text_entry: None,
            mode: Mode::ReleaseNotes,
            exit: false,
        })
    }

    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.exit {
            terminal
                .draw(|frame| self.render(frame))
                .into_diagnostic()?;

            let (mode_state, mode_entries) = match self.mode {
                Mode::ReleaseNotes => (&mut self.release_notes_state, &mut self.release_notes),
                Mode::MigrationGuides => {
                    (&mut self.migration_guide_state, &mut self.migration_guides)
                }
            };

            if let Event::Key(key) = event::read().into_diagnostic()? {
                // If text entry is enabled, capture all input events
                if let Some(text) = &mut self.text_entry {
                    match key.code {
                        KeyCode::Esc => self.text_entry = None,
                        KeyCode::Backspace => {
                            text.pop();
                        }
                        KeyCode::Enter => {
                            if !text.is_empty()
                                && let Some(index) = mode_state.selected()
                            {
                                mode_entries.insert(
                                    index,
                                    Entry::Section {
                                        title: text.clone(),
                                    },
                                );
                            }
                            self.text_entry = None;
                        }
                        KeyCode::Char(c) => text.push(c),
                        _ => {}
                    }

                    continue;
                }

                match key.code {
                    KeyCode::Esc => self.exit = true,
                    KeyCode::Tab => match self.mode {
                        Mode::ReleaseNotes => self.mode = Mode::MigrationGuides,
                        Mode::MigrationGuides => self.mode = Mode::ReleaseNotes,
                    },
                    KeyCode::Down => {
                        if key.modifiers.contains(KeyModifiers::SHIFT)
                            && let Some(index) = mode_state.selected()
                            && index < mode_entries.len() - 1
                        {
                            mode_entries.swap(index, index + 1);
                        }
                        mode_state.select_next();
                    }
                    KeyCode::Up => {
                        if key.modifiers.contains(KeyModifiers::SHIFT)
                            && let Some(index) = mode_state.selected()
                            && index > 0
                        {
                            mode_entries.swap(index, index - 1);
                        }
                        mode_state.select_previous();
                    }
                    KeyCode::Char('+') => {
                        self.text_entry = Some(String::new());
                    }
                    KeyCode::Char('d') => {
                        if let Some(index) = mode_state.selected()
                            && let Entry::Section { .. } = mode_entries[index]
                        {
                            mode_entries.remove(index);
                        }
                    }
                    _ => {}
                }
            }
        }

        self.write_output()
    }

    pub fn render(&mut self, frame: &mut Frame) {
        use Constraint::*;

        let page_area = frame.area().inner(Margin::new(1, 1));
        let [header_area, instructions_area, _, block_area, _, typing_area] = Layout::vertical([
            Length(2), // header
            Length(2), // instructions
            Length(1), // gap
            Fill(1),   // blocks
            Length(1), // gap
            Length(2), // text input
        ])
        .areas(page_area);

        frame.render_widget(self.header(), header_area);
        frame.render_widget(self.instructions(), instructions_area);

        let (title, mode_state, mode_entries) = match self.mode {
            Mode::ReleaseNotes => (
                "Release Notes",
                &mut self.release_notes_state,
                &self.release_notes,
            ),
            Mode::MigrationGuides => (
                "Migration Guides",
                &mut self.migration_guide_state,
                &self.migration_guides,
            ),
        };
        let items = mode_entries.iter().map(|e| e.as_list_entry());
        let list = List::new(items)
            .block(Block::new().title(title).padding(Padding::uniform(1)))
            .highlight_symbol(">>")
            .highlight_style(Color::Green);

        frame.render_stateful_widget(list, block_area, mode_state);

        if let Some(text) = &self.text_entry {
            let text_entry = Paragraph::new(format!("Section Title: {}", text)).fg(Color::Blue);
            frame.render_widget(text_entry, typing_area);
        }
    }

    fn header(&self) -> impl Widget {
        let text = "Content Exporter Tool";
        text.bold().underlined().into_centered_line()
    }

    fn instructions(&self) -> impl Widget {
        let text =
            "▲ ▼ : navigate    shift + ▲ ▼ : re-order    + : insert section    d : delete section    tab : change focus    esc : save and quit";
        Paragraph::new(text)
            .fg(Color::Magenta)
            .centered()
            .wrap(Wrap { trim: false })
    }

    fn write_output(self) -> Result<()> {
        // Write release notes
        let mut file =
            fs::File::create(self.content_dir.join("merged_release_notes.md")).into_diagnostic()?;

        for entry in self.release_notes {
            match entry {
                Entry::Section { title } => write!(file, "# {title}\n\n").into_diagnostic()?,
                Entry::File { metadata, content } => {
                    let title = metadata.title;

                    let authors = metadata
                        .authors
                        .iter()
                        .flatten()
                        .map(|a| format!("\"{a}\""))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let pull_requests = metadata
                        .pull_requests
                        .iter()
                        .map(|n| format!("{}", n))
                        .collect::<Vec<_>>()
                        .join(", ");

                    write!(
                        file,
                        "## {title}\n{{% heading_metadata(authors=[{authors}] prs=[{pull_requests}]) %}}\n{content}\n\n"
                    )
                    .into_diagnostic()?;
                }
            }
        }

        // Write migration guide
        let mut file = fs::File::create(self.content_dir.join("merged_migration_guides.md"))
            .into_diagnostic()?;

        for entry in self.migration_guides {
            match entry {
                Entry::Section { title } => write!(file, "## {title}\n\n").into_diagnostic()?,
                Entry::File { metadata, content } => {
                    let title = metadata.title;

                    let pull_requests = metadata
                        .pull_requests
                        .iter()
                        .map(|n| format!("{}", n))
                        .collect::<Vec<_>>()
                        .join(", ");

                    write!(
                        file,
                        "### {title}\n{{% heading_metadata(prs=[{pull_requests}]) %}}\n{content}\n\n"
                    )
                    .into_diagnostic()?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct Metadata {
    title: String,
    authors: Option<Vec<String>>,
    pull_requests: Vec<u32>,
}

#[derive(Debug)]
enum Entry {
    Section { title: String },
    File { metadata: Metadata, content: String },
}

impl Entry {
    fn as_list_entry(&'_ self) -> ListItem<'_> {
        match self {
            Entry::Section { title } => ListItem::new(title.as_str()).underlined().fg(Color::Blue),
            Entry::File { metadata, .. } => ListItem::new(metadata.title.as_str()),
        }
    }
}

/// Loads release content from files in the specified directory
fn load_content(dir: PathBuf, kind: &'static str) -> Result<Vec<Entry>> {
    let re = Regex::new(r"(?s)^---\s*\n(?<frontmatter>.*?)\s*\n---\s*\n(?<content>.*)").unwrap();

    let mut entries = vec![];

    for dir_entry in fs::read_dir(dir)
        .into_diagnostic()
        .wrap_err("unable to read directory")?
    {
        let dir_entry = dir_entry
            .into_diagnostic()
            .wrap_err(format!("unable to access {} file", kind))?;

        // Skip directories
        if !dir_entry.path().is_file() {
            continue;
        }
        // Skip files with invalid names
        let Ok(file_name) = dir_entry.file_name().into_string() else {
            continue;
        };
        // Skip hidden files (like .gitkeep or .DS_Store)
        if file_name.starts_with(".") {
            continue;
        }

        let file_content = fs::read_to_string(dir_entry.path())
            .into_diagnostic()
            .wrap_err(format!("unable to read {} file", kind))?;

        let caps = re.captures(&file_content).ok_or(diagnostic!(
            "failed to find frontmatter in {} file {}",
            kind,
            file_name
        ))?;

        let frontmatter = caps.name("frontmatter").unwrap().as_str();
        let metadata = serde_yml::from_str::<Metadata>(frontmatter).map_err(|e| ParseError {
            src: NamedSource::new(
                format!("{}", dir_entry.path().display()),
                frontmatter.to_owned(),
            ),
            kind,
            file_name,
            err_span: e.location().map(|l| l.index()),
            error: e,
        })?;
        let content = caps.name("content").unwrap().as_str().to_owned();

        entries.push(Entry::File { metadata, content });
    }

    Ok(entries)
}

#[derive(Diagnostic, Debug, Error)]
#[error("failed to parse metadata in {kind} file {file_name}")]
pub struct ParseError {
    #[source_code]
    src: NamedSource<String>,
    kind: &'static str,
    file_name: String,
    #[label("{error}")]
    err_span: Option<usize>,
    error: serde_yml::Error,
}
