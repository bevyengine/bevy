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
use bevy_input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy_input::{ButtonInput, InputSystems};
use bevy_input_focus::{
    FocusCause, FocusGained, FocusLost, FocusedInput, InputFocus, InputFocusSystems,
};
use bevy_math::Vec2;
use bevy_picking::events::{Drag, Pointer, PointerState, Press, Release};
use bevy_picking::pointer::PointerButton;
use bevy_reflect::Reflect;
use bevy_text::{
    scrollable_text_layout_width, EditableText, EditableTextSystems, PreeditCursor, TextEdit,
    TextLayout, TextLayoutInfo,
};
use bevy_time::{Real, Time};
use bevy_ui::widget::{sync_editable_text_viewports, update_editable_text_layout};
use bevy_ui::UiSystems;
use bevy_ui::{
    widget::TextNodeFlags, ComputedNode, ComputedUiRenderTargetInfo, ContentSize,
    InteractionDisabled, Node, UiGlobalTransform, UiScale,
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

/// Autoscroll speed is proportional to the input size
const AUTOSCROLL_BASE_SPEED: f32 = 0.75;
const AUTOSCROLL_MAX_SPEED: f32 = 2.0;
/// Distance from the input to the point along an axis where `AUTOSCROLL_MAX_SPEED` is reached.
/// Proportional to the input size.
const AUTOSCROLL_RAMP_DISTANCE: f32 = 0.5;

/// Returns `true` if the given keyboard input matches an editing-shortcut
/// character (like the `C` in `Ctrl+C`).
///
/// Shortcut matching uses a hybrid strategy, mirroring the behavior of native
/// text fields and browsers:
///
/// - The layout-aware [`logical_key`](KeyboardInput::logical_key) is checked
///   first, so Latin non-QWERTY layouts (AZERTY, Dvorak, ...) keep their
///   conventional shortcut positions.
/// - If the logical key is not an ASCII character (non-Latin layouts such as
///   Cyrillic, Greek, Arabic or Hebrew), the physical
///   [`key_code`](KeyboardInput::key_code) is used as a layout-independent
///   fallback.
///
/// Matching purely on `logical_key` breaks these shortcuts on non-Latin
/// layouts, while matching purely on `key_code` breaks Latin non-QWERTY
/// conventions. See <https://github.com/bevyengine/bevy/issues/24997>.
fn matches_edit_shortcut(input: &KeyboardInput, character: &str, key_code: KeyCode) -> bool {
    match &input.logical_key {
        Key::Character(c) if c.is_ascii() => c.eq_ignore_ascii_case(character),
        Key::Character(_) => input.key_code == key_code,
        _ => false,
    }
}

/// System that processes keyboard input events into text edit actions for focused [`EditableText`] widgets.
///
/// See [`EditableText`] for more details on the standard mapping from keyboard events to text edit actions
/// used by this system.
///
/// Note that this does not immediately apply the edits; they are queued up in [`EditableText::pending_edits`],
/// and then applied later by the [`apply_text_edits`](`bevy_text::apply_text_edits`) system.
fn on_focused_keyboard_input(
    mut keyboard_input: On<FocusedInput<KeyboardInput>>,
    mut query: Query<&mut EditableText, Without<InteractionDisabled>>,
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
        (COMMAND, Key::Character(_))
            if matches_edit_shortcut(&keyboard_input.input, "a", KeyCode::KeyA) =>
        {
            queue_edit(TextEdit::SelectAll);
        }
        (COMMAND, Key::Character(_))
            if matches_edit_shortcut(&keyboard_input.input, "c", KeyCode::KeyC) =>
        {
            queue_edit(TextEdit::Copy);
        }
        (COMMAND, Key::Character(_))
            if matches_edit_shortcut(&keyboard_input.input, "x", KeyCode::KeyX) =>
        {
            queue_edit(TextEdit::Cut);
        }
        (COMMAND, Key::Character(_))
            if matches_edit_shortcut(&keyboard_input.input, "v", KeyCode::KeyV) =>
        {
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
        #[cfg(target_os = "macos")]
        (COMMAND | SHIFT_COMMAND, Key::ArrowUp) => queue_edit(TextEdit::TextStart(shift_pressed)),
        #[cfg(target_os = "macos")]
        (COMMAND | SHIFT_COMMAND, Key::ArrowDown) => queue_edit(TextEdit::TextEnd(shift_pressed)),
        (NONE | SHIFT, Key::ArrowUp) => queue_edit(TextEdit::Up(shift_pressed)),
        (NONE | SHIFT, Key::ArrowDown) => queue_edit(TextEdit::Down(shift_pressed)),
        #[cfg(not(target_os = "macos"))]
        (CTRL, Key::ArrowUp) => queue_edit(TextEdit::ScrollByLines(-1.0)),
        #[cfg(not(target_os = "macos"))]
        (CTRL, Key::ArrowDown) => queue_edit(TextEdit::ScrollByLines(1.0)),
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
    mut text_input_query: Query<
        (
            &mut EditableText,
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        Without<InteractionDisabled>,
    >,
    keys: Res<ButtonInput<Key>>,
    mut input_focus: ResMut<InputFocus>,
    ui_scale: Res<UiScale>,
) {
    if press.button != PointerButton::Primary {
        return;
    }

    let Ok((mut editable_text, node, target, transform)) = text_input_query.get_mut(press.entity)
    else {
        // The press landed on something that isn't an `EditableText`. Clicking away to blur a
        // focused text input is handled by `PointerFocusPlugin` (in `bevy_input_focus`), which
        // triggers a bubbling `AcquireFocus` that clears focus at the window, so there is nothing
        // to do here.
        return;
    };

    input_focus.set(press.entity, FocusCause::Pressed);

    press.propagate(false);

    if editable_text.is_composing() {
        // The IME is active; all input needs to be routed there, including pointer presses.
        return;
    }
    let Some(local_pos) = transform.try_inverse().and_then(|inverse| {
        let local_pos = inverse
            .transform_point2(press.pointer_location.position * target.scale_factor() / ui_scale.0);
        node.content_box()
            .contains(local_pos)
            .then(|| local_pos - node.content_box().min + editable_text.viewport.offset)
    }) else {
        return;
    };

    match press.count {
        1 => {
            editable_text
                .pending_edits
                .push(if keys.pressed(Key::Shift) {
                    TextEdit::ShiftClickExtension
                } else {
                    TextEdit::MoveToPoint
                }(local_pos));
        }
        2 => editable_text.queue_edit(TextEdit::SelectWordAtPoint(local_pos)),
        _ => editable_text.queue_edit(TextEdit::SelectAll),
    }
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
    )>,
    ui_scale: Res<UiScale>,
    input_focus: Res<InputFocus>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }

    if input_focus.get() != Some(drag.entity) {
        return;
    }

    let Ok((mut editable_text, node, target, transform)) = text_input_query.get_mut(drag.entity)
    else {
        return;
    };

    drag.propagate(false);

    if editable_text.is_composing() {
        // The IME is active; all input needs to be routed there, including pointer drags.
        return;
    }

    let Some(local_point) = transform.try_inverse().map(|inverse| {
        inverse
            .transform_point2(drag.pointer_location.position * target.scale_factor() / ui_scale.0)
    }) else {
        return;
    };

    let clamped_local_point = node.content_box().clamp_point(local_point);
    let current_offset = editable_text.viewport.offset;
    editable_text.queue_edit(TextEdit::ExtendSelectionToPoint(
        clamped_local_point - node.content_box().min + current_offset,
    ));
}

pub(crate) fn text_input_autoscroll_system(
    time: Res<Time<Real>>,
    pointer_state: Res<PointerState>,
    input_focus: Res<InputFocus>,
    mut text_input_query: Query<(
        &mut EditableText,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        &UiGlobalTransform,
        &TextLayoutInfo,
        &TextLayout,
    )>,
    ui_scale: Res<UiScale>,
) {
    let Some(entity) = input_focus.get() else {
        return;
    };
    let Some(pointer_position) = pointer_state
        .pointer_buttons
        .iter()
        .filter(|((_, button), ..)| *button == PointerButton::Primary)
        .find_map(|(_, state)| state.dragging.get(&entity).map(|drag| drag.latest_pos))
    else {
        return;
    };

    let Ok((mut editable_text, node, target, transform, layout_info, text_layout)) =
        text_input_query.get_mut(entity)
    else {
        return;
    };

    if editable_text.is_composing()
        || editable_text
            .pending_edits
            .iter()
            .any(|edit| matches!(edit, TextEdit::ImeSetCompose { value, .. } if !value.is_empty()))
    {
        return;
    }

    if editable_text.viewport.size.cmple(Vec2::ZERO).any() {
        return;
    }

    let Some(local_point) = transform.try_inverse().map(|inverse| {
        inverse.transform_point2(pointer_position * target.scale_factor() / ui_scale.0)
    }) else {
        return;
    };

    let clamped_local_point = node.content_box().clamp_point(local_point);

    // Signed per-axis distance of the pointer from the text viewport
    let signed_distance = local_point - clamped_local_point;
    if signed_distance == Vec2::ZERO {
        return;
    }

    // Distance to scroll on each axis
    let scroll_delta = Vec2::new(
        autoscroll_axis(
            signed_distance.x,
            editable_text.viewport.size.x,
            time.delta_secs(),
        ),
        autoscroll_axis(
            signed_distance.y,
            editable_text.viewport.size.y,
            time.delta_secs(),
        ),
    );

    // Calculate the full text layout size, including space for the cursor.
    let full_layout_size = Vec2::new(
        scrollable_text_layout_width(
            text_layout.linebreak,
            layout_info.size.x,
            editable_text.viewport.size.x,
            layout_info.cursor.map(|(_, rect)| rect),
        ),
        layout_info.size.y,
    );

    let clamped_offset = (editable_text.viewport.offset + scroll_delta).clamp(
        Vec2::ZERO,
        (full_layout_size - editable_text.viewport.size).max(Vec2::ZERO),
    );
    let clamped_scroll_delta = clamped_offset - editable_text.viewport.offset;

    if clamped_scroll_delta == Vec2::ZERO {
        return;
    }

    editable_text.queue_edit(TextEdit::ScrollBy(clamped_scroll_delta));

    // Extend the selection using the post-scroll viewport offset.
    editable_text.queue_edit(TextEdit::ExtendSelectionToPoint(
        clamped_local_point - node.content_box().min + clamped_offset,
    ));
}

fn autoscroll_axis(overflow: f32, view_size: f32, time_delta: f32) -> f32 {
    if overflow == 0. || view_size == 0. {
        return 0.;
    }
    let ramp_distance = (overflow.abs() / (view_size * AUTOSCROLL_RAMP_DISTANCE)).min(1.0);
    let speed =
        AUTOSCROLL_BASE_SPEED + ramp_distance * (AUTOSCROLL_MAX_SPEED - AUTOSCROLL_BASE_SPEED);
    overflow.signum() * view_size * speed * time_delta
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
    )>,
    // TODO: support multiple windows and track which one has focus
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
) {
    let Some(focused) = input_focus.get() else {
        return;
    };
    let Ok((editable_text, node, transform, target)) = editable_text_query.get(focused) else {
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
    let ui_local = parley_local + node.content_box().min - editable_text.viewport.offset;
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

/// Observer that clears in-progress IME composition and
/// collapses the current selection to a caret when an [`EditableText`] loses focus .
///
/// We need to clear the composition on focus loss because IME composition state is not automatically tied to widget focus;
/// the IME remains active until explicitly disabled,
/// even if the focused widget changes to another `EditableText` or to a non-`EditableText`.
///
/// Without this, switching focus between two text inputs leaves stale preedit state on the
/// previous input.
/// The IME stays enabled because both entities are [`EditableText`],
/// and no [`Ime::Disabled`] event is ever fired to trigger the cleanup in [`on_ime_input`].
///
/// A [`TextEdit::CollapseSelection`] is also fired to collapse any active highlighted text
/// selection back to the caret cursor position, preventing text styles from getting stuck in a
/// focused color state.
fn on_focus_lost(trigger: On<FocusLost>, mut editable_text_query: Query<&mut EditableText>) {
    if let Ok(mut editable_text) = editable_text_query.get_mut(trigger.entity) {
        editable_text.queue_edit(TextEdit::clear_ime_compose());
        editable_text.queue_edit(TextEdit::CollapseSelection);
    }
}

/// Marker component for [`EditableText`] widgets that should select all text on focus.
///
/// If a pointer press is what caused the focus, the select all is deferred until
/// pointer release and is only applied if there is no other selection by then.
/// For example, if pointer dragging caused a selection to be made, we don't want
/// to replace it with a select all.
#[derive(Component, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct SelectAllOnFocus;

/// Resource to track when a pointer press caused focus on an [`EditableText`].
/// A corresponding pointer release will select all text if there is no other selection.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct QueuedSelectAll(Option<Entity>);

fn on_focus_select_all(
    focus_gained: On<FocusGained>,
    mut q_text_input: Query<(&mut EditableText, Has<SelectAllOnFocus>)>,
    mut queued_select_all: ResMut<QueuedSelectAll>,
) {
    let target = focus_gained.event_target();
    if let Ok((mut editable_text, select_all_on_focus)) = q_text_input.get_mut(target) {
        match focus_gained.event().cause {
            FocusCause::Pressed => {
                if select_all_on_focus {
                    queued_select_all.0 = Some(target);
                }
            }
            FocusCause::Auto | FocusCause::Navigated => {
                if select_all_on_focus {
                    editable_text.queue_edit(TextEdit::SelectAll);
                }
            }
        }
    }
}

/// `on_focus_select_all` defers selection until pointer release if the focus was gained
/// by a pointer press. This system applies the queued selection.
///
/// Note, that the `Pointer<Release>` does not have to happen on the same entity.
fn apply_queued_select_all(
    mut pointer_releases: MessageReader<Pointer<Release>>,
    mut queued_select_all: ResMut<QueuedSelectAll>,
    mut q_text_input: Query<&mut EditableText, With<SelectAllOnFocus>>,
) {
    let Some(target) = queued_select_all.0 else {
        return;
    };
    for pointer_release in pointer_releases.read() {
        if pointer_release.button == PointerButton::Primary
            && let Ok(mut editable_text) = q_text_input.get_mut(target)
        {
            editable_text.queue_edit(TextEdit::SelectAllIfCollapsed);
            queued_select_all.0 = None;
        }
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
/// in the [`EditableTextSystems`] system set.
pub struct EditableTextInputPlugin;

impl Plugin for EditableTextInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QueuedSelectAll>()
            .add_observer(on_focused_keyboard_input)
            .add_observer(on_pointer_drag)
            .add_observer(on_pointer_press)
            .add_observer(on_focus_lost)
            .add_observer(on_focus_select_all)
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
                    // FocusChangeEvents does not mutate the actual InputFocus;
                    // this is a false positive that can be ignored
                    .ambiguous_with(InputFocusSystems::FocusChangeEvents),
            )
            .add_systems(
                PostUpdate,
                text_input_autoscroll_system
                    .after(sync_editable_text_viewports)
                    .before(EditableTextSystems),
            )
            .add_systems(
                PostUpdate,
                apply_queued_select_all
                    .in_set(UiSystems::PostLayout)
                    .before(update_editable_text_layout),
            );

        // These components cannot be registered in `bevy_text` where `EditableText` is defined,
        // because that would create a circular dependency between `bevy_text` and `bevy_ui`.
        app.register_required_components::<EditableText, Node>()
            .register_required_components::<EditableText, TextNodeFlags>()
            .register_required_components::<EditableText, ContentSize>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::Update;
    use bevy_input::ButtonState;
    use bevy_math::Rect;
    use bevy_picking::{events::DragEntry, pointer::PointerId};
    use core::time::Duration;

    #[test]
    fn autoscroll_speed_is_zero_inside_then_ramps_and_caps() {
        let visible_size = 100.0;

        assert_eq!(autoscroll_axis(0.0, visible_size, 1.0), 0.0);
        assert_eq!(autoscroll_axis(25.0, visible_size, 1.0), 137.5);
        assert_eq!(autoscroll_axis(-25.0, visible_size, 1.0), -137.5);
        assert_eq!(autoscroll_axis(50.0, visible_size, 1.0), 200.0);
        assert_eq!(autoscroll_axis(-100.0, visible_size, 1.0), -200.0);
    }

    #[test]
    fn autoscroll_displacement_is_frame_rate_independent() {
        let overflow = 25.0;
        let view_size = 100.0;

        let one_frame = autoscroll_axis(overflow, view_size, 1.0 / 30.0);
        let two_frames = autoscroll_axis(overflow, view_size, 1.0 / 60.0) * 2.0;

        assert!((one_frame - two_frames).abs() < 1e-5);
    }

    fn autoscroll_app(initial_drag_pos: Vec2) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .init_resource::<PointerState>()
            .init_resource::<UiScale>()
            .add_systems(Update, text_input_autoscroll_system);

        let mut editable_text = EditableText::default();
        editable_text.viewport.size = Vec2::splat(100.0);
        let entity = app
            .world_mut()
            .spawn((
                editable_text,
                ComputedNode {
                    size: Vec2::splat(100.0),
                    ..Default::default()
                },
                ComputedUiRenderTargetInfo::default(),
                UiGlobalTransform::default(),
                TextLayoutInfo {
                    size: Vec2::splat(300.0),
                    ..Default::default()
                },
            ))
            .id();

        app.insert_resource(InputFocus::from_entity(entity));
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .insert(
                entity,
                DragEntry {
                    start_pos: initial_drag_pos,
                    latest_pos: initial_drag_pos,
                },
            );
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs_f32(1.0 / 60.0));

        (app, entity)
    }

    #[test]
    fn active_drag_autoscrolls_until_reentry() {
        let (mut app, entity) = autoscroll_app(Vec2::new(100.0, 0.0));

        app.update();

        assert_eq!(
            app.world()
                .entity(entity)
                .get::<EditableText>()
                .unwrap()
                .pending_edits
                .len(),
            2
        );

        app.world_mut()
            .entity_mut(entity)
            .get_mut::<EditableText>()
            .unwrap()
            .pending_edits
            .clear();
        app.update();

        assert_eq!(
            app.world()
                .entity(entity)
                .get::<EditableText>()
                .unwrap()
                .pending_edits
                .len(),
            2
        );

        app.world_mut()
            .entity_mut(entity)
            .get_mut::<EditableText>()
            .unwrap()
            .pending_edits
            .clear();
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .get_mut(&entity)
            .unwrap()
            .latest_pos = Vec2::ZERO;
        app.update();

        assert!(app
            .world()
            .entity(entity)
            .get::<EditableText>()
            .unwrap()
            .pending_edits
            .is_empty());
    }

    #[test]
    fn autoscroll_into_cursor_space() {
        let (mut app, entity) = autoscroll_app(Vec2::new(100.0, 0.0));
        let mut entity_mut = app.world_mut().entity_mut(entity);
        let mut editable_text = entity_mut.get_mut::<EditableText>().unwrap();
        // set up the viewport with the caret fitting exactly
        editable_text.viewport.reveal_caret(
            Rect::new(295.0, 0.0, 300.0, 20.0),
            Vec2::new(300.0, 300.0),
            Vec2::ZERO,
            core::iter::empty(),
        );
        assert_eq!(editable_text.viewport.offset.x, 200.0);

        // set layout caret outside of viewport
        entity_mut.insert(TextLayout::no_wrap());
        entity_mut.get_mut::<TextLayoutInfo>().unwrap().cursor =
            Some((true, Rect::new(295.0, 0.0, 320.0, 20.0)));

        app.update();

        // expect autoscroll will have queued a scroll right edit
        assert!(matches!(
            app.world().entity(entity).get::<EditableText>().unwrap().pending_edits.first(),
            Some(TextEdit::ScrollBy(delta)) if 0. < delta.x
        ));
    }

    #[test]
    fn autoscroll_stops_after_release_cancellation_or_focus_loss() {
        let (mut app, entity) = autoscroll_app(Vec2::new(100.0, 0.0));
        app.update();

        assert_eq!(
            app.world()
                .entity(entity)
                .get::<EditableText>()
                .unwrap()
                .pending_edits
                .len(),
            2
        );

        app.world_mut()
            .entity_mut(entity)
            .get_mut::<EditableText>()
            .unwrap()
            .pending_edits
            .clear();

        app.insert_resource(InputFocus::default());
        app.update();
        let editable_text = app.world().entity(entity).get::<EditableText>().unwrap();
        assert!(editable_text.pending_edits.is_empty());
    }

    #[test]
    fn ime_composition_stops_autoscroll() {
        let (mut app, entity) = autoscroll_app(Vec2::new(100.0, 0.0));
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<EditableText>()
            .unwrap()
            .queue_edit(TextEdit::ImeSetCompose {
                value: "compose".into(),
                cursor: Some(PreeditCursor {
                    anchor: 0,
                    focus: 7,
                }),
            });

        app.update();

        assert_eq!(
            app.world()
                .entity(entity)
                .get::<EditableText>()
                .unwrap()
                .pending_edits
                .len(),
            1
        );
    }

    fn shortcut_keyboard_input(logical_key: Key, key_code: KeyCode) -> KeyboardInput {
        KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn logical_key_wins_on_latin_layouts() {
        // US QWERTY: logical "c" on the physical `KeyC`.
        let event = shortcut_keyboard_input(Key::Character("c".into()), KeyCode::KeyC);
        assert!(matches_edit_shortcut(&event, "c", KeyCode::KeyC));

        // AZERTY: logical "a" lives on the physical `KeyQ`.
        // The layout convention must win over the physical location.
        let event = shortcut_keyboard_input(Key::Character("a".into()), KeyCode::KeyQ);
        assert!(matches_edit_shortcut(&event, "a", KeyCode::KeyA));
        assert!(!matches_edit_shortcut(&event, "q", KeyCode::KeyQ));
    }

    #[test]
    fn physical_fallback_on_non_latin_layouts() {
        // Cyrillic layout: the key at the `KeyC` position produces Cyrillic "с".
        let event = shortcut_keyboard_input(Key::Character("с".into()), KeyCode::KeyC);
        assert!(matches_edit_shortcut(&event, "c", KeyCode::KeyC));
        // ...but it must not match a different physical key.
        assert!(!matches_edit_shortcut(&event, "a", KeyCode::KeyA));
    }

    #[test]
    fn named_keys_never_match() {
        let event = shortcut_keyboard_input(Key::Enter, KeyCode::Enter);
        assert!(!matches_edit_shortcut(&event, "c", KeyCode::KeyC));
    }
}
