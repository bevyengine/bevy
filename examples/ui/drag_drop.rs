use bevy::{
    input::mouse::{MouseButtonInput, MouseMotion},
    prelude::*,
    render::texture::ImageSettings,
    ui::FocusPolicy,
};

// Markers
#[derive(Component)]
struct Dragable;

#[derive(Component)]
struct DragActive;

#[derive(Component)]
struct DragMoving;

#[derive(Component)]
struct DragDropping;

#[derive(Component)]
struct DragStarting;

#[derive(Component)]
struct SourceDropContainer(pub Entity);

#[derive(Component)]
pub struct DropContainer(bool);

impl Default for DropContainer {
    fn default() -> Self {
        DropContainer(true)
    }
}

struct DragableHoverEvent(pub Entity);
struct DragableClickEvent(pub Entity);
struct DragableDropEvent(pub Entity);

#[derive(Component)]
struct MouseIn;

struct MouseEnterEvent(pub Entity);
struct MouseExitEvent(pub Entity);

struct DragDropPlugin;

pub const CLEAR_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);

impl Plugin for DragDropPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ImageSettings::default_nearest())
            .insert_resource(ClearColor(CLEAR_COLOR))
            .add_plugins(DefaultPlugins)
            .add_startup_system_to_stage(StartupStage::PreStartup, spawn_camera)
            .add_startup_system_to_stage(StartupStage::PostStartup, spawn_scene)
            .add_event::<DragableClickEvent>()
            .add_event::<DragableHoverEvent>()
            .add_event::<DragableDropEvent>()
            .add_event::<MouseEnterEvent>()
            .add_event::<MouseExitEvent>()
            // System order is important
            .add_system(mouse_drag_active_moving)
            .add_system(mouse_drag_active_click)
            .add_system(mouse_drag_not_active)
            .add_system(mouse_hover)
            .add_system(mouse_exit.after(mouse_hover))
            .add_system(mouse_enter.after(mouse_exit))
            .add_system(dragable_drop_event.after(mouse_drag_active_click))
            .add_system(dragable_click_event.after(mouse_drag_not_active))
            .add_system(dragable_starting.after(dragable_click_event));
    }
}

fn main() {
    App::new().add_plugin(DragDropPlugin).run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            color: Color::NONE.into(),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.), Val::Px(225.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    color: Color::rgba(1., 1., 1., 0.01).into(),
                    ..default()
                })
                .insert(Interaction::default())
                .insert(FocusPolicy::default())
                .insert(DropContainer::default())
                .with_children(|parent| {
                    for i in 0..10 {
                        parent
                            .spawn_bundle(
                                TextBundle::from_section(
                                    format!("DragMe {i}!"),
                                    TextStyle {
                                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                                        font_size: 20.,
                                        color: Color::GRAY,
                                    },
                                )
                                .with_style(Style {
                                    size: Size::new(Val::Undefined, Val::Px(25.)),
                                    margin: UiRect {
                                        left: Val::Auto,
                                        right: Val::Auto,
                                        ..default()
                                    },
                                    ..default()
                                }),
                            )
                            .insert(Interaction::default())
                            .insert(FocusPolicy::default())
                            .insert(Dragable);
                    }
                });
            // Drop area

            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Px(500.0), Val::Px(300.0)),
                        position: UiRect::new(
                            Val::Px(50.0),
                            Val::Undefined,
                            Val::Undefined,
                            Val::Undefined,
                        ),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    color: Color::rgba(0.5, 0.1, 0.1, 1.).into(),
                    ..default()
                })
                .insert(Interaction::default())
                .insert(FocusPolicy::default())
                .insert(DropContainer::default())
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle::from_section(
                        "Drop into free space!".to_string(),
                        TextStyle {
                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                            font_size: 20.,
                            color: Color::GRAY,
                        },
                    ));
                });
        });
}

fn mouse_hover(
    mut event_mouse_enter: EventWriter<MouseEnterEvent>,
    mut query_start_hover: Query<(Entity, &Interaction), (Changed<Interaction>, Without<MouseIn>)>,
    mut event_mouse_exit: EventWriter<MouseExitEvent>,
    mut query_stop_hover: Query<(Entity, &Interaction), (Changed<Interaction>, With<MouseIn>)>,
) {
    for (entity, interaction) in query_start_hover.iter_mut() {
        match *interaction {
            Interaction::Clicked | Interaction::None => {}
            Interaction::Hovered => {
                event_mouse_enter.send(MouseEnterEvent(entity));
            }
        }
    }

    for (entity, interaction) in query_stop_hover.iter_mut() {
        match *interaction {
            Interaction::Clicked | Interaction::Hovered => {}
            Interaction::None => {
                event_mouse_exit.send(MouseExitEvent(entity));
            }
        }
    }
}

fn mouse_enter(mut mouse_enter_event: EventReader<MouseEnterEvent>, mut commands: Commands) {
    for mouse_enter in mouse_enter_event.iter() {
        commands.entity(mouse_enter.0).insert(MouseIn);
    }
}

fn mouse_exit(mut mouse_exit_event: EventReader<MouseExitEvent>, mut commands: Commands) {
    for mouse_exit in mouse_exit_event.iter() {
        commands.entity(mouse_exit.0).remove::<MouseIn>();
    }
}

fn mouse_drag_not_active(
    mut query_list: Query<
        (&Interaction, Entity),
        (Changed<Interaction>, Without<DragActive>, With<Dragable>),
    >,
    mut event_hover_dragable: EventWriter<DragableHoverEvent>,
    mut event_click_dragable: EventWriter<DragableClickEvent>,
) {
    for (interaction, entity) in query_list.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                event_click_dragable.send(DragableClickEvent(entity));
            }
            Interaction::Hovered => {
                event_hover_dragable.send(DragableHoverEvent(entity));
            }
            Interaction::None => {}
        }
    }
}

fn mouse_drag_active_moving(
    res_window: Res<Windows>,
    mouse_motion_events: EventReader<MouseMotion>,
    mut query_list: Query<&mut Style, With<DragMoving>>,
) {
    if query_list.is_empty() || mouse_motion_events.is_empty() {
        return;
    }

    if let Some(mouse_position) = res_window.get_primary().unwrap().cursor_position() {
        for style in query_list.iter_mut() {
            update_style_with_mouse_position(style, mouse_position);
        }
    }
}

fn update_style_with_mouse_position(mut style: Mut<Style>, position: Vec2) {
    style.position.bottom = Val::Px(position.y);
    style.position.left = Val::Px(position.x);
    style.position_type = PositionType::Absolute;
}

fn mouse_drag_active_click(
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut event_drop_dragable: EventWriter<DragableDropEvent>,
    query_list: Query<Entity, With<DragActive>>,
) {
    for mouse_event in mouse_button_events.iter() {
        if mouse_event.button == MouseButton::Left && !mouse_event.state.is_pressed() {
            for entity in &query_list {
                event_drop_dragable.send(DragableDropEvent(entity));
            }
        }
    }
}

fn dragable_drop_event(
    mut event_drop_dragable: EventReader<DragableDropEvent>,
    mut query_style: Query<
        (Entity, &mut Style, &Dragable, &SourceDropContainer, &Parent),
        (With<DragMoving>, With<DragActive>),
    >,
    mut commands: Commands,
    query_drop_containers: Query<(Entity, &DropContainer), With<MouseIn>>,
) {
    for event in event_drop_dragable.iter() {
        commands
            .entity(event.0)
            .remove::<DragMoving>()
            .remove::<DragActive>()
            .remove::<FocusPolicy>()
            .insert(Interaction::default())
            .insert(FocusPolicy::Block);

        if let Ok((drag_entity, mut style, _dragable, source_drop_container, parent)) =
            query_style.get_mut(event.0)
        {
            if let Some((drop_container_entity, drop_container)) = query_drop_containers
                .iter()
                .find(|current| current.0.id() != source_drop_container.0.id())
            {
                style.position_type = if drop_container.0 {
                    PositionType::Relative
                } else {
                    PositionType::Absolute
                };
                commands
                    .entity(parent.get())
                    .remove_children(&[drag_entity]);
                commands
                    .entity(drop_container_entity)
                    .add_child(drag_entity);
            } else {
                // Reattach to previous node
                commands
                    .entity(source_drop_container.0)
                    .add_child(drag_entity);
                style.position_type = PositionType::Relative;
            }
            // Reset position
            style.position.bottom = Val::Undefined;
            style.position.left = Val::Undefined;
        }
    }
}

// It may be better to use an event instead
fn dragable_starting(
    query_drag_starting: Query<Entity, With<DragStarting>>,
    mut commands: Commands,
) {
    for entity in query_drag_starting.iter() {
        commands
            .entity(entity)
            .remove::<DragStarting>()
            .insert(DragMoving);
    }
}

fn dragable_click_event(
    mut event_click_dragable: EventReader<DragableClickEvent>,
    mut query_drag_starting: Query<(Entity, &mut Style), Without<DragActive>>,
    mut commands: Commands,
    res_window: Res<Windows>,
    query_root_node: Query<Entity, (With<Node>, Without<Parent>)>,
    query_parent: Query<&Parent, With<Dragable>>,
) {
    let cursor_position_opt = res_window.get_primary().unwrap().cursor_position();
    if cursor_position_opt == Option::None {
        return;
    }

    let cursor_position = cursor_position_opt.unwrap();

    for clicked_entity in event_click_dragable.iter() {
        let current_clicked_entity = query_drag_starting
            .iter_mut()
            .find(|current| current.0.id() == clicked_entity.0.id());

        match current_clicked_entity {
            None => continue,
            Some((entity, style)) => {
                // Remove from parent node
                let parent_entity = query_parent.get(entity).unwrap().get();
                commands.entity(parent_entity).remove_children(&[entity]);

                // Add to root node
                let root = query_root_node.single();
                commands.entity(root).add_child(entity);

                // Modify status
                commands
                    .entity(entity)
                    .remove::<Interaction>()
                    .remove::<FocusPolicy>()
                    .insert(DragActive)
                    .insert(DragStarting)
                    .insert(SourceDropContainer(parent_entity))
                    .insert(FocusPolicy::Pass);

                update_style_with_mouse_position(style, cursor_position);

                // Hint: in order to not let other dropables block your drop event into the dropcontainer, you could remove all FocusPolicies and reapply them ondrop or implement your own mouse collision
            }
        }
    }
}
