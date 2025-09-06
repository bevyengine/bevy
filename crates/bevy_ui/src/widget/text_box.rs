use crate::{
    widget::{measure_lines, update_text_field_attributes, TextSubmission},
    ComputedNode, Node, UiGlobalTransform, UiScale, UiSystems,
};
use bevy_app::{Plugin, PostUpdate};
use bevy_color::{
    palettes::tailwind::{BLUE_900, GRAY_300, GRAY_400, GRAY_950, SKY_300},
    Color,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    event::EventReader,
    lifecycle::HookContext,
    observer::{Observer, On},
    query::Has,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::DeferredWorld,
};
use bevy_input::{
    keyboard::{Key, KeyboardInput},
    mouse::{MouseScrollUnit, MouseWheel},
    ButtonInput,
};
use bevy_input_focus::{tab_navigation::NavAction, FocusedInput, InputFocus};
use bevy_math::{IVec2, Rect, Vec2};
use bevy_picking::{
    events::{Click, Drag, Move, Pointer, Press},
    hover::HoverMap,
    pointer::PointerButton,
};
use bevy_text::{
    Justify, LineBreak, Motion, TextEdit, TextEdits, TextFont, TextInputAttributes,
    TextInputBuffer, TextInputSystems, TextInputTarget,
};
use bevy_time::Time;
use core::time::Duration;

pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<GlobalTextInputState>()
            .init_resource::<TextInputMultiClickPeriod>()
            .add_systems(
                PostUpdate,
                (
                    update_text_box_attributes,
                    update_text_field_attributes,
                    measure_lines,
                    mouse_wheel_scroll,
                )
                    .chain()
                    .in_set(UiSystems::Content),
            )
            .add_systems(
                PostUpdate,
                (update_targets, update_cursor_visibility)
                    .in_set(UiSystems::PostLayout)
                    .before(TextInputSystems),
            );
    }
}

/// Controls how long until the button has to be pressed again to register a multi-click.
#[derive(Resource, Deref, DerefMut)]
pub struct TextInputMultiClickPeriod(pub Duration);

impl Default for TextInputMultiClickPeriod {
    fn default() -> Self {
        Self(Duration::from_secs_f32(0.5))
    }
}

fn update_targets(mut text_input_node_query: Query<(&ComputedNode, &mut TextInputTarget)>) {
    for (node, mut target) in text_input_node_query.iter_mut() {
        let new_target = TextInputTarget {
            size: node.size(),
            scale_factor: node.inverse_scale_factor.recip(),
        };
        target.set_if_neq(new_target);
    }
}

fn update_text_box_attributes(
    mut text_input_node_query: Query<(&TextBox, &TextFont, &mut TextInputAttributes)>,
) {
    for (text_box, font, mut attributes) in text_input_node_query.iter_mut() {
        attributes.set_if_neq(TextInputAttributes {
            font: font.font.clone(),
            font_size: font.font_size,
            font_smoothing: font.font_smoothing,
            justify: text_box.justify,
            line_break: text_box.line_break,
            line_height: font.line_height,
            max_chars: None,
            visible_lines: text_box.lines,
        });
    }
}

/// Single line input.
/// No vertical scrolling, tabs or newlines.
/// Enter submits.
/// Todo: Up and down walk submission history
#[derive(Default, Component)]
pub struct SingleLineInputField;

#[derive(Component, Debug)]
pub struct TextUnderCursorColor(pub Color);

impl Default for TextUnderCursorColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

/// Main text input component
#[derive(Component, Debug, Default)]
#[require(
    Node,
    TextFont,
    TextInputStyle,
    TextInputMultiClickCounter,
    TextInputBuffer,
    TextCursorBlinkTimer
)]
#[component(
    on_add = on_add_text_input_node,
    on_remove = on_remove_input_focus,
)]
pub struct TextBox {
    /// maximum number of chars
    pub max_chars: Option<usize>,
    /// justification
    pub justify: Justify,
    /// line break
    pub line_break: LineBreak,
    /// Number of visible lines
    pub lines: Option<f32>,
    /// Clear text input contents and history on submit
    pub clear_on_submit: bool,
}

fn on_add_text_input_node(mut world: DeferredWorld, context: HookContext) {
    for mut observer in [
        Observer::new(on_text_input_dragged),
        Observer::new(on_text_input_pressed),
        Observer::new(on_multi_click_set_selection),
        Observer::new(on_move_clear_multi_click),
        Observer::new(on_focused_keyboard_input),
    ] {
        observer.watch_entity(context.entity);
        world.commands().spawn(observer);
    }
}

fn on_remove_input_focus(mut world: DeferredWorld, context: HookContext) {
    let mut input_focus = world.resource_mut::<InputFocus>();
    if input_focus.0 == Some(context.entity) {
        input_focus.0 = None;
    }
}

/// Visual styling for a text input widget.
#[derive(Component, Clone)]
pub struct TextInputStyle {
    /// Text color
    pub text_color: Color,
    /// Color of text under an overwrite cursor
    pub overwrite_text_color: Color,
    /// Color of input prompt (if set)
    pub prompt_color: Color,
    /// Color of the cursor.
    pub cursor_color: Color,
    /// Size of the insert cursor relative to the space advance width and line height.
    pub cursor_size: Vec2,
    /// How long the cursor blinks for.
    pub cursor_blink_interval: Duration,
    /// Color of selection blocks
    pub selection_color: Color,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            text_color: GRAY_300.into(),
            overwrite_text_color: GRAY_950.into(),
            prompt_color: SKY_300.into(),
            cursor_color: GRAY_400.into(),
            cursor_size: Vec2::new(0.2, 1.),
            cursor_blink_interval: Duration::from_secs_f32(0.5),
            selection_color: BLUE_900.into(),
        }
    }
}

/// Controls cursor blinking.
/// If the value is none or greater than the `blink_interval` in `TextCursorStyle` then the cursor
/// is not displayed.
#[derive(Component, Debug, Default)]
pub struct TextCursorBlinkTimer(pub Option<f32>);

pub fn update_cursor_visibility(
    time: Res<Time>,
    input_focus: Res<InputFocus>,
    mut query: Query<(
        Entity,
        &TextInputStyle,
        &TextEdits,
        &mut TextCursorBlinkTimer,
    )>,
) {
    for (entity, style, actions, mut timer) in query.iter_mut() {
        timer.0 = if input_focus
            .0
            .is_some_and(|focused_entity| focused_entity == entity)
        {
            Some(if actions.queue.is_empty() {
                (timer.0.unwrap_or(0.) + time.delta_secs())
                    .rem_euclid(style.cursor_blink_interval.as_secs_f32() * 2.)
            } else {
                0.
            })
        } else {
            None
        }
    }
}

/// Text input modifier state
#[derive(Resource, Debug, Default)]
pub struct GlobalTextInputState {
    /// If true typed glyphs overwrite the glyph at the current cursor position, instead of inserting before it.
    pub overwrite: bool,
}

#[derive(Component, Default, Debug)]
pub struct TextInputMultiClickCounter {
    pub(crate) last_click_time: f32,
    pub(crate) click_count: usize,
}

fn on_text_input_pressed(
    trigger: On<Pointer<Press>>,
    mut node_query: Query<(&ComputedNode, &UiGlobalTransform, &mut TextEdits)>,
    mut input_focus: ResMut<InputFocus>,
    ui_scale: Res<UiScale>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut actions)) = node_query.get_mut(trigger.entity()) else {
        return;
    };

    input_focus.set(trigger.entity());

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position = trigger.pointer_location.position * node.inverse_scale_factor().recip()
        / ui_scale.0
        - rect.min;

    actions.queue(TextEdit::Click(position.as_ivec2()));
}

fn on_text_input_dragged(
    trigger: On<Pointer<Drag>>,
    mut node_query: Query<(&ComputedNode, &UiGlobalTransform, &mut TextEdits)>,
    input_focus: Res<InputFocus>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    if input_focus
        .0
        .is_none_or(|input_focus_entity| input_focus_entity != trigger.entity())
    {
        return;
    }

    let Ok((node, transform, mut actions)) = node_query.get_mut(trigger.entity()) else {
        return;
    };

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position =
        trigger.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

    let target = IVec2 {
        x: position.x as i32,
        y: position.y as i32,
    };

    actions.queue(TextEdit::Drag(target));
}

fn on_multi_click_set_selection(
    click: On<Pointer<Click>>,
    time: Res<Time>,
    multi_click_delay: Res<TextInputMultiClickPeriod>,
    mut text_input_nodes: Query<(&ComputedNode, &UiGlobalTransform, &mut TextEdits)>,
    mut multi_click_data: Query<&mut TextInputMultiClickCounter>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut actions)) = text_input_nodes.get_mut(click.entity()) else {
        return;
    };

    let now = time.elapsed_secs();
    if let Ok(mut multi_click_data) = multi_click_data.get_mut(click.entity())
        && now - multi_click_data.last_click_time
            <= multi_click_delay.as_secs_f32() * multi_click_data.click_count as f32
    {
        let rect = Rect::from_center_size(transform.translation, node.size());

        let position =
            click.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

        match multi_click_data.click_count {
            1 => {
                multi_click_data.click_count += 1;
                multi_click_data.last_click_time = now;

                actions.queue(TextEdit::DoubleClick(position.as_ivec2()));
                return;
            }
            2 => {
                actions.queue(TextEdit::TripleClick(position.as_ivec2()));
                if let Ok(mut entity) = commands.get_entity(click.entity()) {
                    entity.try_remove::<TextInputMultiClickCounter>();
                }
                return;
            }
            _ => (),
        }
    }
    if let Ok(mut entity) = commands.get_entity(click.entity()) {
        entity.try_insert(TextInputMultiClickCounter {
            last_click_time: now,
            click_count: 1,
        });
    }
}

fn on_move_clear_multi_click(move_event: On<Pointer<Move>>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(move_event.entity()) {
        entity.try_remove::<TextInputMultiClickCounter>();
    }
}

/// Updates the scroll position of scrollable nodes in response to mouse input
pub fn mouse_wheel_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut node_query: Query<(&TextInputBuffer, &mut TextEdits)>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (_, pointer_map) in hover_map.iter() {
            for (entity, _) in pointer_map.iter() {
                let Ok((buffer, mut actions)) = node_query.get_mut(*entity) else {
                    continue;
                };
                if mouse_wheel_event.y == 0. {
                    continue;
                }
                let lines = match mouse_wheel_event.unit {
                    MouseScrollUnit::Line => mouse_wheel_event.y,
                    MouseScrollUnit::Pixel => {
                        let line_height = buffer.with_buffer(|buffer| buffer.metrics().line_height);
                        line_height / mouse_wheel_event.y
                    }
                };
                actions.queue(TextEdit::Scroll {
                    lines: -lines as i32,
                });
            }
        }
    }
}

pub enum NextFocus {
    Stay,
    Clear,
    Navigate(NavAction),
}

impl Default for NextFocus {
    fn default() -> Self {
        NextFocus::Navigate(NavAction::Next)
    }
}

pub fn on_focused_keyboard_input(
    mut trigger: On<FocusedInput<KeyboardInput>>,
    mut commands: Commands,
    mut query: Query<(
        &mut TextEdits,
        Has<SingleLineInputField>,
        &TextBox,
        &TextInputBuffer,
    )>,
    mut modifiers: Option<ResMut<GlobalTextInputState>>,
    keyboard_state: Res<ButtonInput<Key>>,
) {
    let Ok((mut actions, is_single_line, attribs, buffer)) = query.get_mut(trigger.entity()) else {
        return;
    };

    if !trigger.event().input.state.is_pressed() {
        trigger.propagate(false);
        return;
    }

    let pressed_key = &trigger.event().input;
    let is_shift_pressed = keyboard_state.pressed(Key::Shift);
    #[cfg(not(target_os = "macos"))]
    let is_command_pressed = keyboard_state.pressed(Key::Control);
    #[cfg(target_os = "macos")]
    let is_command_pressed = keyboard_state.pressed(Key::Super);

    if is_command_pressed {
        match &pressed_key.logical_key {
            Key::Character(str) => {
                if let Some(char) = str.chars().next() {
                    // convert to lowercase so that the commands work with capslock on
                    match (char.to_ascii_lowercase(), is_shift_pressed) {
                        ('c', false) => {
                            // copy
                            actions.queue(TextEdit::Copy);
                        }
                        ('x', false) => {
                            // cut
                            actions.queue(TextEdit::Cut);
                        }
                        ('v', false) => {
                            // paste
                            actions.queue(TextEdit::Paste);
                        }
                        ('a', false) => {
                            // select all
                            actions.queue(TextEdit::SelectAll);
                        }
                        _ => {
                            // not recognized, ignore
                        }
                    }
                }
            }
            Key::ArrowLeft => {
                actions.queue(TextEdit::Motion {
                    motion: Motion::PreviousWord,
                    with_select: is_shift_pressed,
                });
            }
            Key::ArrowRight => {
                actions.queue(TextEdit::Motion {
                    motion: Motion::NextWord,
                    with_select: is_shift_pressed,
                });
            }
            Key::ArrowUp => {
                if !is_single_line {
                    actions.queue(TextEdit::Scroll { lines: -1 });
                }
            }
            Key::ArrowDown => {
                if !is_single_line {
                    actions.queue(TextEdit::Scroll { lines: 1 });
                }
            }
            Key::Home => {
                actions.queue(TextEdit::motion(Motion::BufferStart, is_shift_pressed));
            }
            Key::End => {
                actions.queue(TextEdit::motion(Motion::BufferEnd, is_shift_pressed));
            }
            _ => {
                // not recognized, ignore
            }
        }
    } else {
        match &pressed_key.logical_key {
            Key::Character(_) | Key::Space => {
                let str = if let Key::Character(str) = &pressed_key.logical_key {
                    str.chars()
                } else {
                    " ".chars()
                };
                for char in str {
                    actions.queue(
                        if modifiers
                            .as_ref()
                            .is_some_and(|modifiers| modifiers.overwrite)
                        {
                            TextEdit::Overwrite(char)
                        } else {
                            TextEdit::Insert(char)
                        },
                    );
                }
            }
            Key::Enter => {
                if is_single_line || is_shift_pressed {
                    commands.trigger_targets(
                        TextSubmission {
                            text: buffer.get_text(),
                            entity: trigger.entity(),
                        },
                        trigger.entity(),
                    );

                    if attribs.clear_on_submit {
                        actions.queue(TextEdit::Clear);

                        // if let Some(history) = maybe_history.as_mut() {
                        //     history.clear();
                        // }
                    }
                } else {
                    actions.queue(TextEdit::NewLine);
                }
            }
            Key::Backspace => {
                actions.queue(TextEdit::Backspace);
            }
            Key::Delete => {
                if is_shift_pressed {
                    actions.queue(TextEdit::Cut);
                } else {
                    actions.queue(TextEdit::Delete);
                }
            }
            Key::PageUp => {
                actions.queue(TextEdit::motion(Motion::PageUp, is_shift_pressed));
            }
            Key::PageDown => {
                actions.queue(TextEdit::motion(Motion::PageDown, is_shift_pressed));
            }
            Key::ArrowLeft => {
                actions.queue(TextEdit::motion(Motion::Left, is_shift_pressed));
            }
            Key::ArrowRight => {
                actions.queue(TextEdit::motion(Motion::Right, is_shift_pressed));
            }
            Key::ArrowUp => {
                actions.queue(TextEdit::motion(Motion::Up, is_shift_pressed));
            }
            Key::ArrowDown => {
                actions.queue(TextEdit::motion(Motion::Down, is_shift_pressed));
            }
            Key::Home => {
                actions.queue(TextEdit::motion(Motion::Home, is_shift_pressed));
            }
            Key::End => {
                actions.queue(TextEdit::motion(Motion::End, is_shift_pressed));
            }
            Key::Escape => {
                actions.queue(TextEdit::Escape);
            }
            Key::Tab => {
                if !is_single_line {
                    actions.queue(if is_shift_pressed {
                        TextEdit::Unindent
                    } else {
                        TextEdit::Indent
                    });
                } else {
                    trigger.propagate(true);
                    return;
                }
            }
            Key::Insert => {
                if let Some(modifiers) = modifiers.as_mut() {
                    modifiers.overwrite = !modifiers.overwrite;
                }
            }
            _ => {}
        }
    }
    trigger.propagate(false);
}
