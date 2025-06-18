use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::observer::Observer;
use bevy_ecs::world::DeferredWorld;
use bevy_text::input::TextInputCommands;
use bevy_ui::Node;

/// Main text input component
#[derive(Component, Debug, Default)]
#[require(TextInputModifiers, Node)]
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
    trigger: Trigger<Pointer<Pressed>>,
    mut node_query: Query<(
        &ComputedNode,
        &GlobalTransform,
        &mut TextInputBuffer,
        &TextInputNode,
    )>,
    mut text_input_pipeline: ResMut<TextInputPipeline>,
    mut input_focus: ResMut<InputFocus>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut buffer, input)) = node_query.get_mut(trigger.target) else {
        return;
    };

    if !input.is_enabled || !input.focus_on_pointer_down {
        return;
    }

    if !input_focus
        .get()
        .is_some_and(|active_input| active_input == trigger.target)
    {
        input_focus.set(trigger.target);
    }

    let rect = Rect::from_center_size(transform.translation().truncate(), node.size());

    let position =
        trigger.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

    let mut editor = buffer
        .editor
        .borrow_with(&mut text_input_pipeline.font_system);

    let scroll = editor.with_buffer(|buffer| buffer.scroll());

    editor.action(Action::Click {
        x: position.x as i32 + scroll.horizontal as i32,
        y: position.y as i32,
    });
}

fn on_text_input_dragged(
    trigger: Trigger<Pointer<Drag>>,
    mut node_query: Query<(
        &ComputedNode,
        &GlobalTransform,
        &mut TextInputBuffer,
        &TextInputNode,
    )>,
    mut text_input_pipeline: ResMut<TextInputPipeline>,
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

    let Ok((node, transform, mut buffer, input)) = node_query.get_mut(trigger.target) else {
        return;
    };

    if !input.is_enabled || !input.focus_on_pointer_down {
        return;
    }

    let rect = Rect::from_center_size(transform.translation().truncate(), node.size());

    let position =
        trigger.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

    let mut editor = buffer
        .editor
        .borrow_with(&mut text_input_pipeline.font_system);

    let scroll = editor.with_buffer(|buffer| buffer.scroll());

    editor.action(Action::Drag {
        x: position.x as i32 + scroll.horizontal as i32,
        y: position.y as i32,
    });
}

fn on_multi_click_set_selection(
    click: Trigger<Pointer<Click>>,
    time: Res<Time>,
    mut text_input_nodes: Query<(
        &TextInputNode,
        &mut TextInputCommands,
        &mut TextInputBuffer,
        &GlobalTransform,
        &ComputedNode,
    )>,
    mut multi_click_datas: Query<&mut MultiClickData>,
    mut text_input_pipeline: ResMut<TextInputPipeline>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let entity = click.target();

    let Ok((input, mut queue, mut buffer, transform, node)) = text_input_nodes.get_mut(entity)
    else {
        return;
    };

    if !input.is_enabled || !input.focus_on_pointer_down {
        return;
    }

    let now = time.elapsed_secs();
    if let Ok(mut multi_click_data) = multi_click_datas.get_mut(entity) {
        if now - multi_click_data.last_click_time
            <= MULTI_CLICK_PERIOD * multi_click_data.click_count as f32
        {
            let rect = Rect::from_center_size(transform.translation().truncate(), node.size());

            let position =
                click.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;
            let mut editor = buffer
                .editor
                .borrow_with(&mut text_input_pipeline.font_system);
            let scroll = editor.with_buffer(|buffer| buffer.scroll());
            match multi_click_data.click_count {
                1 => {
                    multi_click_data.click_count += 1;
                    multi_click_data.last_click_time = now;

                    queue.add(TextInputAction::Edit(TextInputEdit::DoubleClick {
                        x: position.x as i32 + scroll.horizontal as i32,
                        y: position.y as i32,
                    }));
                    return;
                }
                2 => {
                    editor.action(Action::Motion(Motion::ParagraphStart));
                    let cursor = editor.cursor();
                    editor.set_selection(Selection::Normal(cursor));
                    editor.action(Action::Motion(Motion::ParagraphEnd));
                    if let Ok(mut entity) = commands.get_entity(entity) {
                        entity.try_remove::<MultiClickData>();
                    }
                    return;
                }
                _ => (),
            }
        }
    }
    if let Ok(mut entity) = commands.get_entity(entity) {
        entity.try_insert(MultiClickData {
            last_click_time: now,
            click_count: 1,
        });
    }
}

fn on_move_clear_multi_click(move_: Trigger<Pointer<Move>>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(move_.target()) {
        entity.try_remove::<MultiClickData>();
    }
}

fn on_focused_keyboard_input(
    trigger: Trigger<FocusedInput<KeyboardInput>>,
    mut query: Query<(&TextInputNode, &mut TextInputCommands)>,
    mut global_state: ResMut<TextInputGlobalState>,
) {
    if let Ok((input, mut queue)) = query.get_mut(trigger.target()) {
        let TextInputGlobalState {
            shift,
            overwrite_mode,
            command,
        } = &mut *global_state;
        queue_text_input_action(
            &input.mode,
            shift,
            overwrite_mode,
            command,
            &trigger.event().input,
            |action| {
                queue.add(action);
            },
        );
    }
}
