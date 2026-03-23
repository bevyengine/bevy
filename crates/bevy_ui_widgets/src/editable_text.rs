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

    let control = keys.pressed(Key::Control);
    let super_key = keys.pressed(Key::Super);
    let command = control || super_key;
    let alt = keys.pressed(Key::Alt); // On mac option is mapped to `Key::Alt` by `bevy_input`.
    let shift = keys.pressed(Key::Shift);

    let mut should_propagate = true;

    let mut queue_edit = |edit| {
        if trigger.input.state.is_pressed() {
            editable_text.queue_edit(edit);
        }
        should_propagate = false;
    };

    match (super_key, control, alt, command, &trigger.input.logical_key) {
        (_, _, _, _, Key::Copy) => queue_edit(TextEdit::Copy),
        (_, _, _, _, Key::Cut) => queue_edit(TextEdit::Cut),
        (_, _, _, _, Key::Paste) => queue_edit(TextEdit::Paste),
        (_, _, false, true, Key::Character(c)) if c.eq_ignore_ascii_case("a") => {
            queue_edit(TextEdit::SelectAll)
        }
        (_, _, false, true, Key::Character(c)) if c.eq_ignore_ascii_case("c") => {
            queue_edit(TextEdit::Copy)
        }
        (_, _, false, true, Key::Character(c)) if c.eq_ignore_ascii_case("x") => {
            queue_edit(TextEdit::Cut)
        }
        (_, _, false, true, Key::Character(c)) if c.eq_ignore_ascii_case("v") => {
            queue_edit(TextEdit::Paste)
        }
        (_, true, _, _, Key::Backspace) | (_, _, true, _, Key::Backspace) => {
            queue_edit(TextEdit::BackspaceWord)
        }
        (_, true, _, _, Key::Delete) | (_, _, true, _, Key::Delete) => {
            queue_edit(TextEdit::DeleteWord)
        }
        (true, false, _, _, Key::ArrowLeft) => queue_edit(TextEdit::HardLineStart(shift)),
        (true, false, _, _, Key::ArrowRight) => queue_edit(TextEdit::HardLineEnd(shift)),
        (_, true, _, _, Key::ArrowLeft) | (_, _, true, _, Key::ArrowLeft) => {
            queue_edit(TextEdit::WordLeft(shift))
        }
        (_, true, _, _, Key::ArrowRight) | (_, _, true, _, Key::ArrowRight) => {
            queue_edit(TextEdit::WordRight(shift))
        }
        (_, _, _, _, Key::ArrowLeft) => queue_edit(TextEdit::Left(shift)),
        (_, _, _, _, Key::ArrowRight) => queue_edit(TextEdit::Right(shift)),
        (_, true, _, _, Key::ArrowUp) | (true, _, _, _, Key::ArrowUp) => {
            queue_edit(TextEdit::TextStart(shift))
        }
        (_, true, _, _, Key::ArrowDown) | (true, _, _, _, Key::ArrowDown) => {
            queue_edit(TextEdit::TextEnd(shift))
        }
        (_, _, _, _, Key::ArrowUp) => queue_edit(TextEdit::Up(shift)),
        (_, _, _, _, Key::ArrowDown) => queue_edit(TextEdit::Down(shift)),
        (_, true, _, _, Key::Home) | (true, _, _, _, Key::Home) => {
            queue_edit(TextEdit::TextStart(shift))
        }
        (_, true, _, _, Key::End) | (true, _, _, _, Key::End) => {
            queue_edit(TextEdit::TextEnd(shift))
        }
        (_, _, true, _, Key::Home) => queue_edit(TextEdit::HardLineStart(shift)),
        (_, _, true, _, Key::End) => queue_edit(TextEdit::HardLineEnd(shift)),
        (_, _, _, _, Key::Home) => queue_edit(TextEdit::LineStart(shift)),
        (_, _, _, _, Key::End) => queue_edit(TextEdit::LineEnd(shift)),
        (_, _, _, _, Key::Backspace) => queue_edit(TextEdit::Backspace),
        (_, _, _, _, Key::Delete) => queue_edit(TextEdit::Delete),
        (_, _, _, _, Key::Escape) => queue_edit(TextEdit::CollapseSelection),
        (_, _, _, _, Key::Tab) => {
            // Parley doesn't support tabs yet.
            // Ignore and propagate to allow for tab navigation.
        }
        (_, _, _, false, Key::Character(_)) | (_, _, _, false, Key::Space) => {
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
