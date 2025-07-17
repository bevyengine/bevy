#![allow(missing_docs)]

use crate::ComputedNode;
use crate::Node;
use crate::UiGlobalTransform;
use crate::UiScale;
use crate::UiSystems;
use bevy_app::Plugin;
use bevy_app::PostUpdate;
use bevy_color::palettes::tailwind::GRAY_400;
use bevy_color::Color;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::observer::Observer;
use bevy_ecs::observer::On;
use bevy_ecs::query::With;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::world::DeferredWorld;
use bevy_input::keyboard::Key;
use bevy_input::keyboard::KeyboardInput;
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;
use bevy_input_focus::InputFocus;
use bevy_math::IVec2;
use bevy_math::Rect;
use bevy_picking::events::Click;
use bevy_picking::events::Drag;
use bevy_picking::events::Move;
use bevy_picking::events::Pointer;
use bevy_picking::events::Press;
use bevy_picking::pointer::PointerButton;
use bevy_text::Motion;
use bevy_text::TextColor;
use bevy_text::TextFont;
use bevy_text::TextInputAction;
use bevy_text::TextInputActions;
use bevy_text::TextInputAttributes;
use bevy_text::TextInputBuffer;
use bevy_text::TextInputHistory;
use bevy_text::TextInputSystems;
use bevy_text::TextInputTarget;
use bevy_text::TextLayout;
use bevy_text::TextLayoutInfo;
use bevy_time::Time;

pub struct TextInputNodePlugin;

impl Plugin for TextInputNodePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<TextInputModifiers>()
            .init_resource::<TextInputOverwriteMode>()
            .init_resource::<InputFocus>()
            .add_systems(
                PostUpdate,
                (update_targets, update_attributes, update_cursor_visibility)
                    .after(UiSystems::Layout)
                    .before(TextInputSystems)
                    .before(bevy_text::update_text_input_buffers),
            );
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

fn update_attributes(
    mut text_input_node_query: Query<(&TextFont, &TextLayout, &mut TextInputAttributes)>,
) {
    for (font, layout, mut attributes) in text_input_node_query.iter_mut() {
        attributes.set_if_neq(TextInputAttributes {
            font: font.font.clone(),
            font_size: font.font_size,
            font_smoothing: font.font_smoothing,
            justify: layout.justify,
            line_break: layout.linebreak,
            line_height: font.line_height,
            max_chars: None,
        });
    }
}

#[derive(Resource, Default)]
pub struct TextInputOverwriteMode(pub bool);

/// Main text input component
#[derive(Component, Debug, Default)]
#[require(
    Node,
    TextFont,
    TextColor,
    TextInputMultiClickCounter,
    TextInputBuffer,
    TextInputTarget,
    TextLayout,
    TextInputAttributes,
    TextInputActions,
    TextCursorStyle,
    TextLayoutInfo,
    TextCursorBlinkTimer,
    TextInputHistory
)]
#[component(
    on_add = on_add_text_input_node,
    on_remove = on_remove_input_focus,
)]
pub struct TextInputNode {}

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

#[derive(Component, Debug)]
pub struct TextCursorStyle {
    pub color: Color,
    pub width: f32,
    pub height: f32,
    pub blink_interval: f32,
}

impl Default for TextCursorStyle {
    fn default() -> Self {
        Self {
            color: GRAY_400.into(),
            width: 0.1,
            height: 1.,
            blink_interval: 0.5,
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
        &TextCursorStyle,
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
                (timer.0.unwrap_or(0.) + time.delta_secs()).rem_euclid(style.blink_interval * 2.)
            } else {
                0.
            })
        } else {
            None
        }
    }
}

/// Global text input modifiers
#[derive(Resource, Debug, Default)]
pub struct TextInputModifiers {
    /// true if shift is held down
    pub shift: bool,
    /// true if ctrl or Command key is held down
    pub command: bool,
    /// If true typed glyphs overwrite the glyph at the current cursor position, instead of inserting before it.
    pub overwrite: bool,
}

pub(crate) const MULTI_CLICK_PERIOD: f32 = 0.5; // seconds

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

    if !input_focus
        .0
        .is_some_and(|input_focus_entity| input_focus_entity == trigger.target())
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
    mut text_input_nodes: Query<(&ComputedNode, &UiGlobalTransform, &mut TextInputActions)>,
    mut multi_click_datas: Query<&mut TextInputMultiClickCounter>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut actions)) = text_input_nodes.get_mut(click.target()) else {
        return;
    };

    let now = time.elapsed_secs();
    if let Ok(mut multi_click_data) = multi_click_datas.get_mut(click.target()) {
        if now - multi_click_data.last_click_time
            <= MULTI_CLICK_PERIOD * multi_click_data.click_count as f32
        {
            let rect = Rect::from_center_size(transform.translation, node.size());

            let position =
                click.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;
            // let mut editor = buffer
            //     .editor
            //     .borrow_with(&mut text_input_pipeline.font_system);
            // let scroll = editor.with_buffer(|buffer| buffer.scroll());
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

fn on_focused_keyboard_input(
    trigger: On<FocusedInput<KeyboardInput>>,
    mut query: Query<&mut TextInputActions, With<TextInputNode>>,
    mut modifiers: ResMut<TextInputModifiers>,
    mut overwrite_mode: ResMut<TextInputOverwriteMode>,
) {
    if let Ok(mut actions) = query.get_mut(trigger.target()) {
        let keyboard_input = &trigger.event().input;
        match keyboard_input.logical_key {
            Key::Shift => {
                modifiers.shift = keyboard_input.state == ButtonState::Pressed;
                return;
            }
            Key::Control => {
                modifiers.command = keyboard_input.state == ButtonState::Pressed;
                return;
            }
            #[cfg(target_os = "macos")]
            Key::Super => {
                modifiers.command = keyboard_input.state == ButtonState::Pressed;
                return;
            }
            _ => {}
        };

        if keyboard_input.state.is_pressed() {
            if modifiers.command {
                match &keyboard_input.logical_key {
                    Key::Character(str) => {
                        if let Some(char) = str.chars().next() {
                            // convert to lowercase so that the commands work with capslock on
                            match (char.to_ascii_lowercase(), modifiers.shift) {
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
                                    // not recognised, ignore
                                }
                            }
                        }
                    }
                    Key::ArrowLeft => {
                        actions.queue(TextInputAction::Motion {
                            motion: Motion::PreviousWord,
                            with_select: modifiers.shift,
                        });
                    }
                    Key::ArrowRight => {
                        actions.queue(TextInputAction::Motion {
                            motion: Motion::NextWord,
                            with_select: modifiers.shift,
                        });
                    }
                    Key::ArrowUp => {
                        actions.queue(TextInputAction::Scroll { lines: -1 });
                    }
                    Key::ArrowDown => {
                        actions.queue(TextInputAction::Scroll { lines: 1 });
                    }
                    Key::Home => {
                        actions.queue(TextInputAction::motion(
                            Motion::BufferStart,
                            modifiers.shift,
                        ));
                    }
                    Key::End => {
                        actions.queue(TextInputAction::motion(Motion::BufferEnd, modifiers.shift));
                    }
                    _ => {
                        // not recognised, ignore
                    }
                }
            } else {
                match &keyboard_input.logical_key {
                    Key::Character(_) | Key::Space => {
                        let str = if let Key::Character(str) = &keyboard_input.logical_key {
                            str.chars()
                        } else {
                            " ".chars()
                        };
                        for char in str {
                            actions.queue(if overwrite_mode.0 {
                                TextInputAction::Overwrite(char)
                            } else {
                                TextInputAction::Insert(char)
                            });
                        }
                    }
                    Key::Enter => {
                        actions.queue(TextInputAction::NewLine);
                    }
                    Key::Backspace => {
                        actions.queue(TextInputAction::Backspace);
                    }
                    Key::Delete => {
                        if modifiers.shift {
                            actions.queue(TextInputAction::Cut);
                        } else {
                            actions.queue(TextInputAction::Delete);
                        }
                    }
                    Key::PageUp => {
                        actions.queue(TextInputAction::motion(Motion::PageUp, modifiers.shift));
                    }
                    Key::PageDown => {
                        actions.queue(TextInputAction::motion(Motion::PageDown, modifiers.shift));
                    }
                    Key::ArrowLeft => {
                        actions.queue(TextInputAction::motion(Motion::Left, modifiers.shift));
                    }
                    Key::ArrowRight => {
                        actions.queue(TextInputAction::motion(Motion::Right, modifiers.shift));
                    }
                    Key::ArrowUp => {
                        actions.queue(TextInputAction::motion(Motion::Up, modifiers.shift));
                    }
                    Key::ArrowDown => {
                        actions.queue(TextInputAction::motion(Motion::Down, modifiers.shift));
                    }
                    Key::Home => {
                        actions.queue(TextInputAction::motion(Motion::Home, modifiers.shift));
                    }
                    Key::End => {
                        actions.queue(TextInputAction::motion(Motion::End, modifiers.shift));
                    }
                    Key::Escape => {
                        actions.queue(TextInputAction::Escape);
                    }
                    Key::Tab => {
                        if modifiers.shift {
                            actions.queue(TextInputAction::Unindent);
                        } else {
                            actions.queue(TextInputAction::Indent);
                        }
                    }
                    Key::Insert => {
                        if modifiers.shift {
                            overwrite_mode.0 = !overwrite_mode.0;
                        } else {
                            // paste
                            actions.queue(TextInputAction::Paste);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
