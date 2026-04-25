//! Input handling for [`EditableText`] widgets.
//!
//! This module provides systems to process keyboard input events and apply text edits
//! to focused [`EditableText`] widgets.
//!
//! Note that this module is distinct from the core `bevy_text` crate to avoid pulling in
//! [`bevy_input`] to that crate, which is intended to be usable in non-interactive contexts.

use bevy_a11y::AccessibilitySystems;
use bevy_app::{App, Plugin, PostUpdate, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input::{ButtonInput, InputSystems};
use bevy_input_focus::{FocusLost, FocusedInput, InputFocus, InputFocusSystems};
use bevy_math::Vec2;
use bevy_picking::events::{Drag, Pointer, Press};
use bevy_picking::pointer::PointerButton;
use bevy_text::{EditableText, PreeditCursor, TextEdit};
use bevy_ui::widget::{scroll_editable_text, update_editable_text_layout, TextScroll};
use bevy_ui::UiSystems;
use bevy_ui::{
    widget::TextNodeFlags, ComputedNode, ComputedUiRenderTargetInfo, ContentSize, Node,
    UiGlobalTransform, UiScale,
};
use bevy_window::{Ime, PrimaryWindow, Window};

const NONE: u8 = 0;
const SUPER: u8 = 1;
const CTRL: u8 = 2;
const ALT: u8 = 4;
const SHIFT: u8 = 8;
const COMMAND: u8 = if cfg!(target_os = "macos") {
    SUPER
} else {
    CTRL
};
// Modifier key for word-level navigation and selection. Alt on macOS, Control otherwise.
const WORD: u8 = if cfg!(target_os = "macos") { ALT } else { CTRL };
const SHIFT_WORD: u8 = SHIFT | WORD;
#[cfg(target_os = "macos")]
const SHIFT_SUPER: u8 = SHIFT | SUPER;
const SHIFT_COMMAND: u8 = SHIFT | COMMAND;
#[cfg(not(target_os = "macos"))]
const SHIFT_ALT: u8 = SHIFT | ALT;

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

    // While the IME is composing, all keyboard input (including Tab) belongs to the IME.
    // Stopping propagation prevents `handle_tab_navigation` (registered on the window,
    // which sits further up the `FocusedInput` bubble chain) from responding to Tab;
    // inadvertently changing focus while the IME is active.
    if editable_text.is_composing() {
        keyboard_input.propagate(false);
        return;
    }

    let allow_newlines = editable_text.allow_newlines;

    // Bitflags representing states of modifier keys.
    // On macOS Option is mapped to `Key::Alt` by `bevy_input`.
    let mod_flags = (SUPER * u8::from(keys.pressed(Key::Super)))
        | (CTRL * u8::from(keys.pressed(Key::Control)))
        | (ALT * u8::from(keys.pressed(Key::Alt)))
        | (SHIFT * u8::from(keys.pressed(Key::Shift)));

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
            queue_edit(TextEdit::SelectAll);
        }
        (COMMAND, Key::Character(c)) if c.eq_ignore_ascii_case("c") => {
            queue_edit(TextEdit::Copy);
        }
        (COMMAND, Key::Character(c)) if c.eq_ignore_ascii_case("x") => queue_edit(TextEdit::Cut),
        (COMMAND, Key::Character(c)) if c.eq_ignore_ascii_case("v") => {
            queue_edit(TextEdit::Paste);
        }
        #[cfg(not(target_os = "macos"))]
        (SHIFT, Key::Delete) => queue_edit(TextEdit::Cut),
        (WORD, Key::Backspace) => queue_edit(TextEdit::BackspaceWord),
        (WORD, Key::Delete) => queue_edit(TextEdit::DeleteWord),
        #[cfg(target_os = "macos")]
        (SUPER | SHIFT_SUPER, Key::ArrowLeft) => queue_edit(TextEdit::HardLineStart(shift_pressed)),
        #[cfg(target_os = "macos")]
        (SUPER | SHIFT_SUPER, Key::ArrowRight) => queue_edit(TextEdit::HardLineEnd(shift_pressed)),
        #[cfg(not(target_os = "macos"))]
        (ALT | SHIFT_ALT, Key::Home) => queue_edit(TextEdit::HardLineStart(shift_pressed)),
        #[cfg(not(target_os = "macos"))]
        (ALT | SHIFT_ALT, Key::End) => queue_edit(TextEdit::HardLineEnd(shift_pressed)),
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
        (NONE, Key::Enter) if allow_newlines => {
            queue_edit(TextEdit::Insert("\n".into()));
        }
        _ => {
            // Ignore and propagate to allow for tab navigation and submit actions.
        }
    }

    keyboard_input.propagate(should_propagate);
}

/// System that processes pointer press events into text edit actions for [`EditableText`] widgets.
///
/// Note that this does not immediately apply the edits; they are queued up in [`EditableText::pending_edits`],
/// and then applied later by the [`apply_text_edits`](`bevy_text::apply_text_edits`) system.
fn on_pointer_press(
    mut press: On<Pointer<Press>>,
    mut text_input_query: Query<(
        &mut EditableText,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        &UiGlobalTransform,
        &TextScroll,
    )>,
    keys: Res<ButtonInput<Key>>,
    mut input_focus: ResMut<InputFocus>,
    ui_scale: Res<UiScale>,
) {
    if press.button != PointerButton::Primary {
        return;
    }

    let Ok((mut editable_text, node, target, transform, text_scroll)) =
        text_input_query.get_mut(press.entity)
    else {
        return;
    };

    if editable_text.is_composing() {
        // The IME is active; all input needs to be routed there, including pointer presses.
        return;
    }

    let Some(local_pos) = transform.try_inverse().map(|inverse| {
        inverse
            .transform_point2(press.pointer_location.position * target.scale_factor() / ui_scale.0)
            - node.content_box().min
            + text_scroll.0
    }) else {
        return;
    };

    editable_text
        .pending_edits
        .push(if keys.pressed(Key::Shift) {
            TextEdit::ShiftClickExtension
        } else {
            TextEdit::MoveToPoint
        }(local_pos));

    input_focus.set(press.entity);

    press.propagate(false);
}

/// System that processes pointer drag events into text edit actions for [`EditableText`] widgets.
///
/// Note that this does not immediately apply the edits; they are queued up in [`EditableText::pending_edits`],
/// and then applied later by the [`apply_text_edits`](`bevy_text::apply_text_edits`) system.
fn on_pointer_drag(
    mut drag: On<Pointer<Drag>>,
    mut text_input_query: Query<(
        &mut EditableText,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        &UiGlobalTransform,
        &TextScroll,
    )>,
    ui_scale: Res<UiScale>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }

    let Ok((mut editable_text, node, target, transform, text_scroll)) =
        text_input_query.get_mut(drag.entity)
    else {
        return;
    };

    if editable_text.is_composing() {
        // The IME is active; all input needs to be routed there, including pointer drags.
        return;
    }

    let Some(local_pos) = transform.try_inverse().map(|inverse| {
        inverse
            .transform_point2(drag.pointer_location.position * target.scale_factor() / ui_scale.0)
            - node.content_box().min
            + text_scroll.0
    }) else {
        return;
    };

    editable_text
        .pending_edits
        .push(TextEdit::ExtendSelectionToPoint(local_pos));
    drag.propagate(false);
}

/// System that processes [`Ime`] events into [`TextEdit`] actions for the focused [`EditableText`] widget.
///
/// Preedit text (in-progress IME composition) is excluded from [`EditableText::value`].
/// On commit, the preedit is cleared and the committed string is inserted.
///
/// Note that this does not immediately apply the edits; they are queued up in [`EditableText::pending_edits`],
/// and then applied later by the [`apply_text_edits`](`bevy_text::apply_text_edits`) system.
fn on_ime_input(
    mut ime_reader: MessageReader<Ime>,
    input_focus: Res<InputFocus>,
    mut editable_text_query: Query<&mut EditableText>,
) {
    let Some(focused_entity) = input_focus.get() else {
        // No focused entity, nothing to do.
        // Still need to drain the reader to prevent stale events on next focus.
        ime_reader.read().for_each(drop);
        return;
    };

    let Ok(mut editable_text) = editable_text_query.get_mut(focused_entity) else {
        // Focused entity is not an EditableText, nothing to do.
        // Still need to drain the reader to prevent stale events on next focus.
        ime_reader.read().for_each(drop);
        return;
    };

    for ime in ime_reader.read() {
        match ime {
            Ime::Preedit { value, cursor, .. } => {
                editable_text.queue_edit(TextEdit::ImeSetCompose {
                    value: value.as_str().into(),
                    cursor: cursor.map(|(anchor, focus)| PreeditCursor { anchor, focus }),
                });
            }
            Ime::Commit { value, .. } => {
                editable_text.queue_edit(TextEdit::ImeCommit {
                    value: value.as_str().into(),
                });
            }
            Ime::Disabled { .. } => {
                // IME was force-disabled; cancel any in-progress composition.
                editable_text.queue_edit(TextEdit::clear_ime_compose());
            }
            Ime::Enabled { .. } => {
                // Defensively clear any stale compose state before a fresh composition starts.
                editable_text.queue_edit(TextEdit::clear_ime_compose());
            }
        }
    }
}

/// System that updates the IME candidate window position to track the text cursor.
///
/// Reads the cursor bounding area from parley's layout and transforms it to screen coordinates
/// so the OS places the IME candidate popup near the active cursor.
///
/// The position reported to the OS is the bottom-left of the cursor/preedit area so the candidate box appears below the
/// composing text rather than overlapping it.
fn update_ime_position(
    input_focus: Res<InputFocus>,
    editable_text_query: Query<(
        &EditableText,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
        &TextScroll,
    )>,
    // TODO: support multiple windows and track which one has focus
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
) {
    let Some(focused) = input_focus.get() else {
        return;
    };
    let Ok((editable_text, node, transform, target, text_scroll)) =
        editable_text_query.get(focused)
    else {
        return;
    };

    let Ok(mut window) = windows.single_mut() else {
        return;
    };

    let area = editable_text.editor.ime_cursor_area();
    // area is in parley text-layout space (origin at top-left of the text layout).
    // Use `y1` (bottom edge) so the OS-drawn candidate box sits below the current line
    // rather than overlapping it.
    let parley_local = Vec2::new(area.x0 as f32, area.y1 as f32);
    let ui_local = parley_local + node.content_box().min - text_scroll.0;
    window.ime_position =
        transform.affine().transform_point2(ui_local) * ui_scale.0 / target.scale_factor();
}

/// System that enables or disables IME on the primary window based on whether the focused entity
/// is an [`EditableText`].
///
/// IME is enabled when an `EditableText` gains focus and disabled when focus moves elsewhere.
fn listen_for_ime_input_when_text_input_focused(
    input_focus: Res<InputFocus>,
    editable_text_query: Query<(), With<EditableText>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !input_focus.is_changed() {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let editable_text_focused = input_focus
        .get()
        .is_some_and(|e| editable_text_query.contains(e));

    // The IME should be enabled whenever an EditableText is focused,
    // even if the IME isn't currently composing (i.e. there's no preedit text).
    // The field here is about "should" we accept IME input, not "are we currently" accepting IME input
    window.ime_enabled = editable_text_focused;
}

/// Observer that clears in-progress IME composition when an [`EditableText`] loses focus.
///
/// We need to clear the composition on focus loss because IME composition state is not automatically tied to widget focus;
/// the IME remains active until explicitly disabled, even if the focused widget changes to another `EditableText` or to a non-`EditableText`.
///
/// Without this, switching focus between two text inputs leaves stale preedit state on the
/// previous input.
/// The IME stays enabled because both entities are [`EditableText`],
/// and no [`Ime::Disabled`] event is ever fired to trigger the cleanup in [`on_ime_input`].
fn on_focus_lost(trigger: On<FocusLost>, mut editable_text_query: Query<&mut EditableText>) {
    if let Ok(mut editable_text) = editable_text_query.get_mut(trigger.entity) {
        editable_text.queue_edit(TextEdit::clear_ime_compose());
    }
}

/// System sets for IME-related systems used by [`EditableTextInputPlugin`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImeSystems {
    /// Processes [`Ime`] events into [`TextEdit`] actions for the focused [`EditableText`].
    ///
    /// Runs in [`PreUpdate`].
    HandleEvents,
    /// Enables or disables IME on the window based on focus state.
    ///
    /// Runs in [`PreUpdate`].
    ToggleWindowIMEInput,
    /// Updates the IME candidate window position to track the text cursor.
    ///
    /// Runs in [`PostUpdate`].
    UpdatePosition,
}

/// Enables support for the [`EditableText`] widget.
///
/// Contains the systems and observers necessary to update widget state and handle user input.
///
/// This plugin is included in the [`UiWidgetsPlugins`](crate::UiWidgetsPlugins) group, but can also be added individually
/// if only editable text input is needed.
///
/// Note that [`TextEdit`]s are applied during [`PostUpdate`]
/// in the [`EditableTextSystems`](bevy_text::EditableTextSystems) system set.
pub struct EditableTextInputPlugin;

impl Plugin for EditableTextInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_focused_keyboard_input)
            .add_observer(on_pointer_drag)
            .add_observer(on_pointer_press)
            .add_observer(on_focus_lost)
            .add_systems(
                PreUpdate,
                (
                    on_ime_input.in_set(ImeSystems::HandleEvents),
                    listen_for_ime_input_when_text_input_focused
                        .in_set(ImeSystems::ToggleWindowIMEInput),
                )
                    .after(InputSystems)
                    .after(InputFocusSystems::Dispatch)
                    .after(UiSystems::Focus),
            )
            .add_systems(
                PostUpdate,
                update_ime_position
                    .in_set(ImeSystems::UpdatePosition)
                    .in_set(UiSystems::PostLayout)
                    .before(AccessibilitySystems::Update)
                    .after(update_editable_text_layout)
                    .after(scroll_editable_text)
                    // FocusChangeEvents does not mutate the actual InputFocus;
                    // this is a false positive that can be ignored
                    .ambiguous_with(InputFocusSystems::FocusChangeEvents),
            );

        // These components cannot be registered in `bevy_text` where `EditableText` is defined,
        // because that would create a circular dependency between `bevy_text` and `bevy_ui`.
        app.register_required_components::<EditableText, Node>()
            .register_required_components::<EditableText, TextNodeFlags>()
            .register_required_components::<EditableText, ContentSize>()
            .register_required_components::<EditableText, TextScroll>();
    }
}
