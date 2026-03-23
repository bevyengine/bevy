//! Input handling for [`EditableText`] widgets.
//!
//! This module provides systems to process keyboard input events and apply text edits
//! to focused [`EditableText`] widgets.
//!
//! Note that this module is distinct from the core `bevy_text` crate to avoid pulling in
//! [`bevy_input`] to that crate, which is intended to be usable in non-interactive contexts.

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input::ButtonInput;
use bevy_input_focus::FocusedInput;
use bevy_text::{EditableText, TextEdit};
use bevy_ui::{widget::TextNodeFlags, ContentSize, Node};

/// System that processes keyboard input events into text edit actions for focused [`EditableText`] widgets.
///
/// See [`EditableText`] for more details on the standard mapping from keyboard events to text edit actions
/// used by this system.
///
/// Note that this does not immediately apply the edits; they are queued up in [`EditableText::pending_edits`],
/// and then applied later by the [`apply_text_edits`](`bevy_text::apply_text_edits`) system.
fn on_focused_keyboard_input(
    mut keyboard_input: On<FocusedInput<KeyboardInput>>,
    mut query: Query<&mut EditableText>,
    keys: Res<ButtonInput<Key>>,
) {
    let Ok(mut editable_text) = query.get_mut(keyboard_input.focused_entity) else {
        return; // Focused entity is not an EditableText, nothing to do
    };

    const NONE: u8 = 0;
    const SUPER: u8 = 1;
    const CTRL: u8 = 2;
    const ALT: u8 = 4;
    const SHIFT: u8 = 8;
    // Super on macOS, Control otherwise.
    const COMMAND: u8 = 16;
    // Modifier key for word-level navigation and selection. Alt on macOS, Control otherwise.
    const WORD: u8 = 32;
    const SHIFT_SUPER: u8 = SHIFT | SUPER;
    const SHIFT_COMMAND: u8 = SHIFT | COMMAND;
    const SHIFT_ALT: u8 = SHIFT | ALT;
    const SHIFT_WORD: u8 = SHIFT | WORD;

    let (command_mod_key, word_mod_key) = if cfg!(target_os = "macos") {
        (Key::Super, Key::Alt)
    } else {
        (Key::Control, Key::Control)
    };

    // Bitflags representing states of modifier keys.
    // On macOS Option is mapped to `Key::Alt` by `bevy_input`.
    let mod_flags = SUPER * u8::from(keys.pressed(Key::Super))
        | CTRL * u8::from(keys.pressed(Key::Control))
        | ALT * u8::from(keys.pressed(Key::Alt))
        | SHIFT * u8::from(keys.pressed(Key::Shift))
        | COMMAND * (u8::from(keys.pressed(command_mod_key)))
        | WORD * (u8::from(keys.pressed(word_mod_key)));

    let shift_pressed = (mod_flags & SHIFT) != 0;

    let mut should_propagate = true;

    let mut queue_edit = |edit| {
        if keyboard_input.input.state.is_pressed() {
            editable_text.queue_edit(edit);
        }
        should_propagate = false;
    };

    match (mod_flags, &keyboard_input.input.logical_key) {
        (NONE, Key::Copy) => queue_edit(TextEdit::Copy),
        (NONE, Key::Cut) => queue_edit(TextEdit::Cut),
        (NONE, Key::Paste) => queue_edit(TextEdit::Paste),
        (COMMAND, Key::Character(c)) if c.eq_ignore_ascii_case("a") => {
            queue_edit(TextEdit::SelectAll)
        }
        (COMMAND, Key::Character(c)) if c.eq_ignore_ascii_case("c") => queue_edit(TextEdit::Copy),
        (COMMAND, Key::Character(c)) if c.eq_ignore_ascii_case("x") => queue_edit(TextEdit::Cut),
        (COMMAND, Key::Character(c)) if c.eq_ignore_ascii_case("v") => queue_edit(TextEdit::Paste),
        (WORD, Key::Backspace) => queue_edit(TextEdit::BackspaceWord),
        (WORD, Key::Delete) => queue_edit(TextEdit::DeleteWord),
        (SUPER | SHIFT_SUPER, Key::ArrowLeft) => queue_edit(TextEdit::HardLineStart(shift_pressed)),
        (SUPER | SHIFT_SUPER, Key::ArrowRight) => queue_edit(TextEdit::HardLineEnd(shift_pressed)),
        (WORD | SHIFT_WORD, Key::ArrowLeft) => queue_edit(TextEdit::WordLeft(shift_pressed)),
        (WORD | SHIFT_WORD, Key::ArrowRight) => queue_edit(TextEdit::WordRight(shift_pressed)),
        (NONE | SHIFT, Key::ArrowLeft) => queue_edit(TextEdit::Left(shift_pressed)),
        (NONE | SHIFT, Key::ArrowRight) => queue_edit(TextEdit::Right(shift_pressed)),
        (COMMAND | SHIFT_COMMAND, Key::ArrowUp) => queue_edit(TextEdit::TextStart(shift_pressed)),
        (COMMAND | SHIFT_COMMAND, Key::ArrowDown) => queue_edit(TextEdit::TextEnd(shift_pressed)),
        (NONE | SHIFT, Key::ArrowUp) => queue_edit(TextEdit::Up(shift_pressed)),
        (NONE | SHIFT, Key::ArrowDown) => queue_edit(TextEdit::Down(shift_pressed)),
        (COMMAND | SHIFT_COMMAND, Key::Home) => queue_edit(TextEdit::TextStart(shift_pressed)),
        (COMMAND | SHIFT_COMMAND, Key::End) => queue_edit(TextEdit::TextEnd(shift_pressed)),
        (ALT | SHIFT_ALT, Key::Home) => queue_edit(TextEdit::HardLineStart(shift_pressed)),
        (ALT | SHIFT_ALT, Key::End) => queue_edit(TextEdit::HardLineEnd(shift_pressed)),
        (NONE | SHIFT, Key::Home) => queue_edit(TextEdit::LineStart(shift_pressed)),
        (NONE | SHIFT, Key::End) => queue_edit(TextEdit::LineEnd(shift_pressed)),
        (NONE, Key::Backspace) => queue_edit(TextEdit::Backspace),
        (NONE, Key::Delete) => queue_edit(TextEdit::Delete),
        (NONE, Key::Escape) => queue_edit(TextEdit::CollapseSelection),
        (NONE | SHIFT, Key::Character(_)) | (NONE, Key::Space) => {
            if let Some(text) = &keyboard_input.input.text
                && !text.is_empty()
            {
                queue_edit(TextEdit::Insert(text.clone()));
            }
        }
        (NONE, Key::Tab) => {
            // Parley doesn't support tabs yet.
            // Ignore and propagate to allow for tab navigation.
        }
        (NONE, Key::Enter) => {
            // Todo, Enter needs handling for special cases.
            // Ignore and propagate to allow for submit.
        }
        _ => {}
    }

    keyboard_input.propagate(should_propagate);
}

/// Enables support for the [`EditableText`] widget.
///
/// Contains the systems and observers necessary to update widget state and handle user input.
///
/// This plugin is included in the [`UiWidgetsPlugins`](crate::UiWidgetsPlugins) group, but can also be added individually
/// if only editable text input is needed.
///
/// Note that [`TextEdit`]s are applied during [`PostUpdate`](bevy_app::PostUpdate)
/// in the [`EditableTextSystems`](bevy_text::EditableTextSystems) system set.
pub struct EditableTextInputPlugin;

impl Plugin for EditableTextInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_focused_keyboard_input);

        // These components cannot be registered in `bevy_text` where `EditableText` is defined,
        // because that would create a circular dependency between `bevy_text` and `bevy_ui`.
        app.register_required_components::<EditableText, Node>()
            .register_required_components::<EditableText, TextNodeFlags>()
            .register_required_components::<EditableText, ContentSize>();
    }
}
