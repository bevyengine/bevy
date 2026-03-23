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
    mut trigger: On<FocusedInput<KeyboardInput>>,
    mut query: Query<&mut EditableText>,
    keys: Res<ButtonInput<Key>>,
) {
    let Ok(mut editable_text) = query.get_mut(trigger.focused_entity) else {
        return; // Focused entity is not an EditableText, nothing to do
    };

    const SUPER: u8 = 1 << 0;
    const CTRL: u8 = 1 << 1;
    const ALT: u8 = 1 << 2;
    const SHIFT: u8 = 1 << 3;
    const COMMAND: u8 = SUPER | CTRL;
    const WORD_MODS: u8 = CTRL | ALT;

    // Bit flags representing states of modifier keys
    // On mac option is mapped to `Key::Alt` by `bevy_input`.
    let mod_flags = SUPER * u8::from(keys.pressed(Key::Super))
        | CTRL * u8::from(keys.pressed(Key::Control))
        | ALT * u8::from(keys.pressed(Key::Alt))
        | SHIFT * u8::from(keys.pressed(Key::Shift));
    let shift = (mod_flags & SHIFT) != 0;

    let mut should_propagate = true;

    let mut queue_edit = |edit| {
        if trigger.input.state.is_pressed() {
            editable_text.queue_edit(edit);
        }
        should_propagate = false;
    };

    match (mod_flags, &trigger.input.logical_key) {
        (_, Key::Copy) => queue_edit(TextEdit::Copy),
        (_, Key::Cut) => queue_edit(TextEdit::Cut),
        (_, Key::Paste) => queue_edit(TextEdit::Paste),
        (m, Key::Character(c))
            if (m & COMMAND) != 0 && (m & (ALT | SHIFT)) == 0 && c.eq_ignore_ascii_case("a") =>
        {
            queue_edit(TextEdit::SelectAll)
        }
        (m, Key::Character(c))
            if (m & COMMAND) != 0 && (m & (ALT | SHIFT)) == 0 && c.eq_ignore_ascii_case("c") =>
        {
            queue_edit(TextEdit::Copy)
        }
        (m, Key::Character(c))
            if (m & COMMAND) != 0 && (m & (ALT | SHIFT)) == 0 && c.eq_ignore_ascii_case("x") =>
        {
            queue_edit(TextEdit::Cut)
        }
        (m, Key::Character(c))
            if (m & COMMAND) != 0 && (m & (ALT | SHIFT)) == 0 && c.eq_ignore_ascii_case("v") =>
        {
            queue_edit(TextEdit::Paste)
        }
        (m, Key::Backspace) if (m & WORD_MODS) != 0 => queue_edit(TextEdit::BackspaceWord),
        (m, Key::Delete) if (m & WORD_MODS) != 0 => queue_edit(TextEdit::DeleteWord),
        (m, Key::ArrowLeft) if (m & SUPER) != 0 && (m & CTRL) == 0 => {
            queue_edit(TextEdit::HardLineStart(shift))
        }
        (m, Key::ArrowRight) if (m & SUPER) != 0 && (m & CTRL) == 0 => {
            queue_edit(TextEdit::HardLineEnd(shift))
        }
        (m, Key::ArrowLeft) if (m & WORD_MODS) != 0 => queue_edit(TextEdit::WordLeft(shift)),
        (m, Key::ArrowRight) if (m & WORD_MODS) != 0 => queue_edit(TextEdit::WordRight(shift)),
        (_, Key::ArrowLeft) => queue_edit(TextEdit::Left(shift)),
        (_, Key::ArrowRight) => queue_edit(TextEdit::Right(shift)),
        (m, Key::ArrowUp) if (m & COMMAND) != 0 => queue_edit(TextEdit::TextStart(shift)),
        (m, Key::ArrowDown) if (m & COMMAND) != 0 => queue_edit(TextEdit::TextEnd(shift)),
        (_, Key::ArrowUp) => queue_edit(TextEdit::Up(shift)),
        (_, Key::ArrowDown) => queue_edit(TextEdit::Down(shift)),
        (m, Key::Home) if (m & COMMAND) != 0 => queue_edit(TextEdit::TextStart(shift)),
        (m, Key::End) if (m & COMMAND) != 0 => queue_edit(TextEdit::TextEnd(shift)),
        (m, Key::Home) if (m & ALT) != 0 => queue_edit(TextEdit::HardLineStart(shift)),
        (m, Key::End) if (m & ALT) != 0 => queue_edit(TextEdit::HardLineEnd(shift)),
        (_, Key::Home) => queue_edit(TextEdit::LineStart(shift)),
        (_, Key::End) => queue_edit(TextEdit::LineEnd(shift)),
        (_, Key::Backspace) => queue_edit(TextEdit::Backspace),
        (_, Key::Delete) => queue_edit(TextEdit::Delete),
        (_, Key::Escape) => queue_edit(TextEdit::CollapseSelection),
        (_, Key::Tab) => {
            // Parley doesn't support tabs yet.
            // Ignore and propagate to allow for tab navigation.
        }
        (m, Key::Character(_)) | (m, Key::Space) if (m & COMMAND) == 0 => {
            if let Some(text) = &trigger.input.text
                && !text.is_empty()
            {
                queue_edit(TextEdit::Insert(text.clone()));
            }
        }
        _ => {}
    }

    trigger.propagate(should_propagate);
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
