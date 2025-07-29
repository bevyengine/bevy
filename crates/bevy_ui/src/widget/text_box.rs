use crate::widget::measure_lines;
use crate::widget::update_text_field_attributes;
use crate::ComputedNode;
use crate::Node;
use crate::UiGlobalTransform;
use crate::UiScale;
use crate::UiSystems;
use bevy_app::Plugin;
use bevy_app::PostUpdate;
use bevy_color::palettes::tailwind::BLUE_900;
use bevy_color::palettes::tailwind::GRAY_300;
use bevy_color::palettes::tailwind::GRAY_400;
use bevy_color::palettes::tailwind::GRAY_950;
use bevy_color::palettes::tailwind::SKY_300;
use bevy_color::Color;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::event::EventReader;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::observer::Observer;
use bevy_ecs::observer::On;
use bevy_ecs::query::Has;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::world::DeferredWorld;
use bevy_input::keyboard::Key;
use bevy_input::keyboard::KeyboardInput;
use bevy_input::mouse::MouseScrollUnit;
use bevy_input::mouse::MouseWheel;
use bevy_input::ButtonInput;
use bevy_input_focus::tab_navigation::NavAction;
use bevy_input_focus::FocusedInput;
use bevy_input_focus::InputFocus;
use bevy_math::IVec2;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_picking::events::Click;
use bevy_picking::events::Drag;
use bevy_picking::events::Move;
use bevy_picking::events::Pointer;
use bevy_picking::events::Press;
use bevy_picking::hover::HoverMap;
use bevy_picking::pointer::PointerButton;
use bevy_text::Justify;
use bevy_text::LineBreak;
use bevy_text::Motion;
use bevy_text::TextFont;
use bevy_text::TextInputAction;
use bevy_text::TextInputActions;
use bevy_text::TextInputAttributes;
use bevy_text::TextInputBuffer;
use bevy_text::TextInputSystems;
use bevy_text::TextInputTarget;
use bevy_text::TextInputUndoHistory;
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
                )
                    .chain()
                    .after(UiSystems::Prepare)
                    .before(UiSystems::Layout),
            )
            .add_systems(
                PostUpdate,
                (mouse_wheel_scroll, update_targets, update_cursor_visibility)
                    .after(UiSystems::Layout)
                    .before(TextInputSystems)
                    .before(bevy_text::update_text_input_buffers),
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
            lines: text_box.lines,
            clear_on_submit: text_box.clear_on_submit,
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
    TextCursorBlinkTimer,
    TextInputUndoHistory
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
        &TextInputActions,
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
    mut node_query: Query<(&ComputedNode, &UiGlobalTransform, &mut TextInputActions)>,
    mut input_focus: ResMut<InputFocus>,
    ui_scale: Res<UiScale>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut actions)) = node_query.get_mut(trigger.target()) else {
        return;
    };

    input_focus.set(trigger.target());

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position = trigger.pointer_location.position * node.inverse_scale_factor().recip()
        / ui_scale.0
        - rect.min;

    actions.queue(TextInputAction::Click(position.as_ivec2()));
}

fn on_text_input_dragged(
    trigger: On<Pointer<Drag>>,
    mut node_query: Query<(&ComputedNode, &UiGlobalTransform, &mut TextInputActions)>,
    input_focus: Res<InputFocus>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    if input_focus
        .0
        .is_none_or(|input_focus_entity| input_focus_entity != trigger.target())
    {
        return;
    }

    let Ok((node, transform, mut actions)) = node_query.get_mut(trigger.target()) else {
        return;
    };

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position =
        trigger.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

    let target = IVec2 {
        x: position.x as i32,
        y: position.y as i32,
    };

    actions.queue(TextInputAction::Drag(target));
}

fn on_multi_click_set_selection(
    click: On<Pointer<Click>>,
    time: Res<Time>,
    multi_click_delay: Res<TextInputMultiClickPeriod>,
    mut text_input_nodes: Query<(&ComputedNode, &UiGlobalTransform, &mut TextInputActions)>,
    mut multi_click_data: Query<&mut TextInputMultiClickCounter>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut actions)) = text_input_nodes.get_mut(click.target()) else {
        return;
    };

    let now = time.elapsed_secs();
    if let Ok(mut multi_click_data) = multi_click_data.get_mut(click.target()) {
        if now - multi_click_data.last_click_time
            <= multi_click_delay.as_secs_f32() * multi_click_data.click_count as f32
        {
            let rect = Rect::from_center_size(transform.translation, node.size());

            let position =
                click.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

            match multi_click_data.click_count {
                1 => {
                    multi_click_data.click_count += 1;
                    multi_click_data.last_click_time = now;

                    actions.queue(TextInputAction::DoubleClick(position.as_ivec2()));
                    return;
                }
                2 => {
                    actions.queue(TextInputAction::TripleClick(position.as_ivec2()));
                    if let Ok(mut entity) = commands.get_entity(click.target()) {
                        entity.try_remove::<TextInputMultiClickCounter>();
                    }
                    return;
                }
                _ => (),
            }
        }
    }
    if let Ok(mut entity) = commands.get_entity(click.target()) {
        entity.try_insert(TextInputMultiClickCounter {
            last_click_time: now,
            click_count: 1,
        });
    }
}

fn on_move_clear_multi_click(move_event: On<Pointer<Move>>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(move_event.target()) {
        entity.try_remove::<TextInputMultiClickCounter>();
    }
}

/// Updates the scroll position of scrollable nodes in response to mouse input
pub fn mouse_wheel_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut node_query: Query<(&TextInputBuffer, &mut TextInputActions)>,
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
                actions.queue(TextInputAction::Scroll {
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
    mut query: Query<(&mut TextInputActions, Has<SingleLineInputField>)>,
    mut modifiers: Option<ResMut<GlobalTextInputState>>,
    keyboard_state: Res<ButtonInput<Key>>,
) {
    let Ok((mut actions, is_single_line)) = query.get_mut(trigger.target()) else {
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
                            actions.queue(TextInputAction::Copy);
                        }
                        ('x', false) => {
                            // cut
                            actions.queue(TextInputAction::Cut);
                        }
                        ('v', false) => {
                            // paste
                            actions.queue(TextInputAction::Paste);
                        }
                        ('z', false) => {
                            actions.queue(TextInputAction::Undo);
                        }
                        #[cfg(target_os = "macos")]
                        ('z', true) => {
                            actions.queue(TextInputAction::Redo);
                        }
                        #[cfg(not(target_os = "macos"))]
                        ('y', false) => {
                            actions.queue(TextInputAction::Redo);
                        }
                        ('a', false) => {
                            // select all
                            actions.queue(TextInputAction::SelectAll);
                        }
                        _ => {
                            // not recognized, ignore
                        }
                    }
                }
            }
            Key::ArrowLeft => {
                actions.queue(TextInputAction::Motion {
                    motion: Motion::PreviousWord,
                    with_select: is_shift_pressed,
                });
            }
            Key::ArrowRight => {
                actions.queue(TextInputAction::Motion {
                    motion: Motion::NextWord,
                    with_select: is_shift_pressed,
                });
            }
            Key::ArrowUp => {
                if !is_single_line {
                    actions.queue(TextInputAction::Scroll { lines: -1 });
                }
            }
            Key::ArrowDown => {
                if !is_single_line {
                    actions.queue(TextInputAction::Scroll { lines: 1 });
                }
            }
            Key::Home => {
                actions.queue(TextInputAction::motion(
                    Motion::BufferStart,
                    is_shift_pressed,
                ));
            }
            Key::End => {
                actions.queue(TextInputAction::motion(Motion::BufferEnd, is_shift_pressed));
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
                            TextInputAction::Overwrite(char)
                        } else {
                            TextInputAction::Insert(char)
                        },
                    );
                }
            }
            Key::Enter => {
                if is_single_line || is_shift_pressed {
                    actions.queue(TextInputAction::Submit);
                } else {
                    actions.queue(TextInputAction::NewLine);
                }
            }
            Key::Backspace => {
                actions.queue(TextInputAction::Backspace);
            }
            Key::Delete => {
                if is_shift_pressed {
                    actions.queue(TextInputAction::Cut);
                } else {
                    actions.queue(TextInputAction::Delete);
                }
            }
            Key::PageUp => {
                actions.queue(TextInputAction::motion(Motion::PageUp, is_shift_pressed));
            }
            Key::PageDown => {
                actions.queue(TextInputAction::motion(Motion::PageDown, is_shift_pressed));
            }
            Key::ArrowLeft => {
                actions.queue(TextInputAction::motion(Motion::Left, is_shift_pressed));
            }
            Key::ArrowRight => {
                actions.queue(TextInputAction::motion(Motion::Right, is_shift_pressed));
            }
            Key::ArrowUp => {
                actions.queue(TextInputAction::motion(Motion::Up, is_shift_pressed));
            }
            Key::ArrowDown => {
                actions.queue(TextInputAction::motion(Motion::Down, is_shift_pressed));
            }
            Key::Home => {
                actions.queue(TextInputAction::motion(Motion::Home, is_shift_pressed));
            }
            Key::End => {
                actions.queue(TextInputAction::motion(Motion::End, is_shift_pressed));
            }
            Key::Escape => {
                actions.queue(TextInputAction::Escape);
            }
            Key::Tab => {
                if !is_single_line {
                    actions.queue(if is_shift_pressed {
                        TextInputAction::Unindent
                    } else {
                        TextInputAction::Indent
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
