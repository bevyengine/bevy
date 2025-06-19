use crate::ComputedNode;
use crate::Node;
use crate::UiGlobalTransform;
use crate::UiScale;
use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::observer::Observer;
use bevy_ecs::observer::On;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::world::DeferredWorld;
use bevy_input::keyboard::KeyboardInput;
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
use bevy_text::TextInputBuffer;
use bevy_text::TextInputCommand;
use bevy_text::TextInputCommands;
use bevy_text::TextPipeline;
use bevy_time::Time;

/// Main text input component
#[derive(Component, Debug, Default)]
#[require(Node, TextInputModifiers, TextInputMultiClickCounter)]
pub struct TextInputNode {}

/// Text input modifiers
#[derive(Component, Debug, Default)]
pub struct TextInputModifiers {
    /// true if shift is held down
    pub shift: bool,
    /// true if ctrl or Command key is held down
    pub command: bool,
    /// If true typed glyphs overwrite the glyph at the current cursor position, instead of inserting before it.
    pub overwrite: bool,
}

const MULTI_CLICK_PERIOD: f32 = 0.5; // seconds

#[derive(Component, Default, Debug)]
pub struct TextInputMultiClickCounter {
    last_click_time: f32,
    click_count: usize,
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

fn on_text_input_pressed(
    trigger: On<Pointer<Press>>,
    mut node_query: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        &mut TextInputBuffer,
        &TextInputNode,
        &mut TextInputCommands,
    )>,
    mut text_input_pipeline: ResMut<TextPipeline>,
    mut input_focus: ResMut<InputFocus>,
    ui_scale: Res<UiScale>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut buffer, input, mut edits)) = node_query.get_mut(trigger.target)
    else {
        return;
    };

    if !input_focus
        .get()
        .is_some_and(|active_input| active_input == trigger.target)
    {
        input_focus.set(trigger.target);
    }

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position = trigger.pointer_location.position * node.inverse_scale_factor().recip()
        / ui_scale.0
        - rect.min;

    edits.queue(TextInputCommand::Click(position.as_ivec2()));
}

fn on_text_input_dragged(
    trigger: On<Pointer<Drag>>,
    mut node_query: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        &mut TextInputBuffer,
        &TextInputNode,
        &mut TextInputCommands,
    )>,
    mut text_input_pipeline: ResMut<TextPipeline>,
    input_focus: Res<InputFocus>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    if !input_focus
        .0
        .is_some_and(|input_focus_entity| input_focus_entity == trigger.target)
    {
        return;
    }

    let Ok((node, transform, mut buffer, input, mut edits)) = node_query.get_mut(trigger.target)
    else {
        return;
    };

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position =
        trigger.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

    let target = IVec2 {
        x: position.x as i32,
        y: position.y as i32,
    };

    edits.queue(TextInputCommand::Drag(target));
}

fn on_multi_click_set_selection(
    click: On<Pointer<Click>>,
    time: Res<Time>,
    mut text_input_nodes: Query<(
        &TextInputNode,
        &mut TextInputCommands,
        &mut TextInputBuffer,
        &UiGlobalTransform,
        &ComputedNode,
    )>,
    mut multi_click_datas: Query<&mut TextInputMultiClickCounter>,
    mut text_input_pipeline: ResMut<TextPipeline>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Some(entity) = click.target() else {
        return;
    };

    let Ok((input, mut edits, mut buffer, transform, node)) = text_input_nodes.get_mut(entity)
    else {
        return;
    };

    let now = time.elapsed_secs();
    if let Ok(mut multi_click_data) = multi_click_datas.get_mut(entity) {
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

                    edits.queue(TextInputCommand::DoubleClick(position.as_ivec2()));
                    return;
                }
                2 => {
                    edits.queue(TextInputCommand::SelectLine);
                    if let Ok(mut entity) = commands.get_entity(entity) {
                        entity.try_remove::<TextInputMultiClickCounter>();
                    }
                    return;
                }
                _ => (),
            }
        }
    }
    if let Ok(mut entity) = commands.get_entity(entity) {
        entity.try_insert(TextInputMultiClickCounter {
            last_click_time: now,
            click_count: 1,
        });
    }
}

fn on_move_clear_multi_click(move_: On<Pointer<Move>>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(move_.target()) {
        entity.try_remove::<TextInputMultiClickCounter>();
    }
}

fn on_focused_keyboard_input(
    trigger: On<FocusedInput<KeyboardInput>>,
    mut query: Query<(
        &TextInputNode,
        &mut TextInputCommands,
        &mut TextInputModifiers,
    )>,
) {
    if let Ok((input, mut queue, mut modifiers)) = query.get_mut(trigger.target()) {
        let keyboard_input = &trigger.event().input;
        match keyboard_input.logical_key {
            Key::Shift => {
                modifiers.shift_pressed = keyboard_input.state == ButtonState::Pressed;
                return;
            }
            Key::Control => {
                modifiers.command_pressed = keyboard_input.state == ButtonState::Pressed;
                return;
            }
            #[cfg(target_os = "macos")]
            Key::Super => {
                modifiers.command_pressed = keyboard_input.state == ButtonState::Pressed;
                return;
            }
            _ => {}
        };

        if keyboard_input.state.is_pressed() {
            if modifiers.command_pressed {
                match &keyboard_input.logical_key {
                    Key::Character(str) => {
                        if let Some(char) = str.chars().next() {
                            // convert to lowercase so that the commands work with capslock on
                            match (char.to_ascii_lowercase(), *shift_pressed) {
                                ('c', false) => {
                                    // copy
                                    queue(TextInputAction::Copy);
                                }
                                ('x', false) => {
                                    // cut
                                    queue(TextInputAction::Cut);
                                }
                                ('v', false) => {
                                    // paste
                                    queue(TextInputAction::Paste);
                                }
                                ('z', false) => {
                                    queue(TextInputAction::Edit(TextInputEdit::Undo));
                                }
                                #[cfg(target_os = "macos")]
                                ('z', true) => {
                                    queue(TextInputAction::Edit(TextInputEdit::Redo));
                                }
                                #[cfg(not(target_os = "macos"))]
                                ('y', false) => {
                                    queue(TextInputAction::Edit(TextInputEdit::Redo));
                                }
                                ('a', false) => {
                                    // select all
                                    queue(TextInputAction::Edit(TextInputEdit::SelectAll));
                                }
                                _ => {
                                    // not recognised, ignore
                                }
                            }
                        }
                    }
                    Key::ArrowLeft => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::PreviousWord,
                            *shift_pressed,
                        )));
                    }
                    Key::ArrowRight => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::NextWord,
                            *shift_pressed,
                        )));
                    }
                    Key::ArrowUp => {
                        if matches!(input_mode, TextInputMode::MultiLine { .. }) {
                            queue(TextInputAction::Edit(TextInputEdit::Scroll { lines: -1 }));
                        }
                    }
                    Key::ArrowDown => {
                        if matches!(input_mode, TextInputMode::MultiLine { .. }) {
                            queue(TextInputAction::Edit(TextInputEdit::Scroll { lines: 1 }));
                        }
                    }
                    Key::Home => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::BufferStart,
                            *shift_pressed,
                        )));
                    }
                    Key::End => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::BufferEnd,
                            *shift_pressed,
                        )));
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
                            queue(TextInputAction::Edit(TextInputEdit::Insert(
                                char,
                                *overwrite_mode,
                            )));
                        }
                    }
                    Key::Enter => match (*shift_pressed, input_mode) {
                        (false, TextInputMode::MultiLine { .. }) => {
                            queue(TextInputAction::Edit(TextInputEdit::Enter));
                        }
                        _ => {
                            queue(TextInputAction::Submit);
                        }
                    },
                    Key::Backspace => {
                        queue(TextInputAction::Edit(TextInputEdit::Backspace));
                    }
                    Key::Delete => {
                        if *shift_pressed {
                            queue(TextInputAction::Cut);
                        } else {
                            queue(TextInputAction::Edit(TextInputEdit::Delete));
                        }
                    }
                    Key::PageUp => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::PageUp,
                            *shift_pressed,
                        )));
                    }
                    Key::PageDown => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::PageDown,
                            *shift_pressed,
                        )));
                    }
                    Key::ArrowLeft => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::Left,
                            *shift_pressed,
                        )));
                    }
                    Key::ArrowRight => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::Right,
                            *shift_pressed,
                        )));
                    }
                    Key::ArrowUp => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::Up,
                            *shift_pressed,
                        )));
                    }
                    Key::ArrowDown => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::Down,
                            *shift_pressed,
                        )));
                    }
                    Key::Home => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::Home,
                            *shift_pressed,
                        )));
                    }
                    Key::End => {
                        queue(TextInputAction::Edit(TextInputEdit::Motion(
                            Motion::End,
                            *shift_pressed,
                        )));
                    }
                    Key::Escape => {
                        queue(TextInputAction::Edit(TextInputEdit::Escape));
                    }
                    Key::Tab => {
                        if matches!(input_mode, TextInputMode::MultiLine { .. }) {
                            if *shift_pressed {
                                queue(TextInputAction::Edit(TextInputEdit::Unindent));
                            } else {
                                queue(TextInputAction::Edit(TextInputEdit::Indent));
                            }
                        }
                    }
                    Key::Insert => {
                        if !*shift_pressed {
                            *overwrite_mode = !*overwrite_mode;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
