//! Input handling for [`EditableText`] widgets.
//!
//! This module provides systems to process keyboard input events and apply text edits
//! to focused [`EditableText`] widgets.
//!
//! Only entities that are focused via the [`InputFocus`] resource will receive keyboard input events.
//!
//! Note that this module is distinct from the core `bevy_text` crate to avoid pulling in
//! [`bevy_input`] to that crate, which is intended to be usable in non-interactive contexts.

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input::InputSystems;
use bevy_input_focus::{InputFocus, InputFocusSystems};
use bevy_text::{EditableText, TextEdit};
use bevy_ui::{widget::TextNodeFlags, ContentSize, Node};

/// System that processes keyboard input events into text edit actions for focused [`EditableText`] widgets.
///
/// See [`EditableText`] for more details on the standard mapping from keyboard events to text edit actions
/// used by this system.
///
/// Note that this does not immediately apply the edits; they are queued up in [`EditableText::pending_edits`],
/// and then applied later by the [`apply_text_edits`](`bevy_text::apply_text_edits`) system.
pub fn process_text_inputs(
    focus: Res<InputFocus>,
    mut query: Query<&mut EditableText>,
    mut keyboard_input: MessageReader<KeyboardInput>,
) {
    // Check if any EditableText is focused
    let focused_entity = if let Some(entity) = focus.get() {
        entity
    } else {
        return; // No focused entity, nothing to do
    };

    let mut editable_text = if let Ok(editable_text) = query.get_mut(focused_entity) {
        editable_text
    } else {
        return; // Focused entity is not an EditableText, nothing to do
    };

    for keyboard_event in keyboard_input.read() {
        match keyboard_event {
            KeyboardInput {
                logical_key: Key::Character(c),
                state: bevy_input::ButtonState::Pressed,
                ..
            } => {
                editable_text.queue_edit(TextEdit::Insert(c.clone()));
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
                editable_text.queue_edit(TextEdit::MoveCursorRight);
            }
            KeyboardInput {
                logical_key: Key::ArrowLeft,
                state: bevy_input::ButtonState::Pressed,
                ..
            } => {
                editable_text.queue_edit(TextEdit::MoveCursorLeft);
            }
            _ => {}
        }
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
        app.add_systems(
            PreUpdate,
            process_text_inputs
                .after(InputFocusSystems::Dispatch)
                .after(InputSystems),
        );

        // These components cannot be registered in `bevy_text` where `EditableText` is defined,
        // because that would create a circular dependency between `bevy_text` and `bevy_ui`.
        app.register_required_components::<EditableText, Node>()
            .register_required_components::<EditableText, TextNodeFlags>()
            .register_required_components::<EditableText, ContentSize>();
    }
}
