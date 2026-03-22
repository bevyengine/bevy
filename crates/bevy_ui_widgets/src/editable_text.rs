//! Input handling for [`EditableText`] widgets.
//!
//! This module provides systems to process keyboard input events and apply text edits
//! to focused [`EditableText`] widgets.
//!
//! Note that this module is distinct from the core `bevy_text` crate to avoid pulling in
//! [`bevy_input`] to that crate, which is intended to be usable in non-interactive contexts.

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy_input::ButtonInput;
use bevy_input_focus::FocusedInput;
use bevy_text::{EditableText, TextEdit};
use bevy_ui::{widget::TextNodeFlags, ContentSize, Node};
use smol_str::SmolStr;

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
    keyboard_button_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut editable_text) = query.get_mut(trigger.focused_entity) else {
        return; // Focused entity is not an EditableText, nothing to do
    };

    let shift = keyboard_button_input.pressed(KeyCode::ShiftLeft)
        || keyboard_button_input.pressed(KeyCode::ShiftRight);

    let mut consumed = true;
    match &trigger.input {
        KeyboardInput {
            logical_key: Key::Character(c),
            state: bevy_input::ButtonState::Pressed,
            ..
        } => {
            editable_text.queue_edit(TextEdit::Insert(c.clone()));
        }
        KeyboardInput {
            logical_key: Key::Space,
            state: bevy_input::ButtonState::Pressed,
            ..
        } => {
            editable_text.queue_edit(TextEdit::Insert(SmolStr::new_inline(" ")));
        }
        KeyboardInput {
            logical_key: Key::Backspace,
            state: bevy_input::ButtonState::Pressed,
            ..
        } => {
            editable_text.queue_edit(TextEdit::Backspace);
        }
        KeyboardInput {
            logical_key: Key::Delete,
            state: bevy_input::ButtonState::Pressed,
            ..
        } => {
            editable_text.queue_edit(TextEdit::Delete);
        }
        KeyboardInput {
            logical_key: Key::ArrowRight,
            state: bevy_input::ButtonState::Pressed,
            ..
        } => {
            if shift {
                editable_text.queue_edit(TextEdit::SelectRight);
            } else {
                editable_text.queue_edit(TextEdit::MoveCursorRight);
            }
        }
        KeyboardInput {
            logical_key: Key::ArrowLeft,
            state: bevy_input::ButtonState::Pressed,
            ..
        } => {
            if shift {
                editable_text.queue_edit(TextEdit::SelectLeft);
            } else {
                editable_text.queue_edit(TextEdit::MoveCursorLeft);
            }
        }
        _ => {
            consumed = false;
        }
    }

    if consumed {
        trigger.propagate(false);
    }
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
