use std::collections::VecDeque;

use bevy_ecs::component::Component;
use bevy_math::IVec2;
use cosmic_text::Buffer;
use cosmic_text::Editor;
use cosmic_text::Metrics;
use cosmic_text::Motion;

/// Text input buffer
#[derive(Component, Debug)]
pub struct TextInputBuffer {
    pub editor: Editor<'static>,
}

impl Default for TextInputBuffer {
    fn default() -> Self {
        Self {
            editor: Editor::new(Buffer::new_empty(Metrics::new(20.0, 20.0))),
        }
    }
}

/// Text input commands queue
#[derive(Component)]
pub struct TextInputCommands {
    pub commands_queue: VecDeque<TextInputCommand>,
}

/// Text input commands
pub enum TextInputCommand {
    Submit,
    Copy,
    Cut,
    Paste,
    /// Move the cursor with some motion
    Motion {
        motion: Motion,
        select: bool,
    },
    Insert(char),
    Overwrite(char),
    Enter,
    Backspace,
    Delete,
    Indent,
    Unindent,
    Click(IVec2),
    DoubleClick(IVec2),
    TripleClick(IVec2),
    Drag(IVec2),
    Scroll {
        lines: i32,
    },
    Undo,
    Redo,
    SelectAll,
}

pub fn apply_text_input_commands() {}
