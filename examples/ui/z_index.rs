//! Demonstrates how to use z-index
//! Shows two colored buttons with transparent text.

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(grab)
        .add_system(update_z_index_text)
        .add_system_to_stage(CoreStage::PreUpdate, grabbed_move)
        .run();
}

#[derive(Component)]
struct ZIndexText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

    // prepare a stack node at the root that's above the stack container
    spawn_stack_node(
        &mut commands,
        font_handle.clone(),
        Color::DARK_GREEN,
        Some(ZIndex::Global(1)),
        UiRect {
            left: Val::Px(20.0),
            bottom: Val::Px(80.0),
            ..default()
        },
    );

    // prepare a stack node at the root that's under the stack container
    spawn_stack_node(
        &mut commands,
        font_handle.clone(),
        Color::ORANGE,
        Some(ZIndex::Global(-1)),
        UiRect {
            left: Val::Px(20.0),
            bottom: Val::Px(20.0),
            ..default()
        },
    );

    // prepare a stack of nodes that can be moved around inside their container.
    let mut stacked_nodes = (0..9)
        .map(|i| {
            spawn_stack_node(
                &mut commands,
                font_handle.clone(),
                Color::rgb(0.1 + i as f32 * 0.1, 0.0, 0.0),
                Some(ZIndex::Local(-4 + i)),
                UiRect {
                    left: Val::Px(10.0 + (i as f32 * 47.5)),
                    bottom: Val::Px(10.0 + (i as f32 * 22.5)),
                    ..default()
                },
            )
        })
        .collect::<Vec<_>>();

    // add a node that has no z-index
    stacked_nodes.push(spawn_stack_node(
        &mut commands,
        font_handle.clone(),
        Color::PURPLE,
        None,
        UiRect {
            left: Val::Px(10.0),
            bottom: Val::Px(120.0),
            ..default()
        },
    ));

    // add a node with a global z-index
    stacked_nodes.push(spawn_stack_node(
        &mut commands,
        font_handle.clone(),
        Color::PINK,
        Some(ZIndex::Global(2)),
        UiRect {
            left: Val::Px(10.0),
            bottom: Val::Px(180.0),
            ..default()
        },
    ));

    // spawn the stack container
    commands
        .spawn_bundle(NodeBundle {
            color: Color::GRAY.into(),
            style: Style {
                size: Size::new(Val::Px(500.0), Val::Px(250.0)),
                overflow: Overflow::Hidden,
                margin: UiRect::all(Val::Auto),
                ..default()
            },
            ..default()
        })
        .push_children(&stacked_nodes);
}

fn spawn_stack_node(
    commands: &mut Commands,
    font_handle: Handle<Font>,
    color: Color,
    z_index: Option<ZIndex>,
    position: UiRect,
) -> Entity {
    let text = commands
        .spawn_bundle(TextBundle::from_section(
            "",
            TextStyle {
                color: Color::WHITE,
                font: font_handle,
                font_size: 20.0,
            },
        ))
        .insert(ZIndexText)
        .id();

    let node = commands
        .spawn_bundle(ButtonBundle {
            color: color.into(),
            style: Style {
                position_type: PositionType::Absolute,
                position,
                size: Size::new(Val::Px(100.0), Val::Px(50.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .insert(Grab)
        .add_child(text)
        .id();

    if let Some(z_index) = z_index {
        commands.entity(node).insert(z_index);
    }

    node
}

fn update_z_index_text(
    mut text_query: Query<(&Parent, &mut Text), With<ZIndexText>>,
    zindex_query: Query<&ZIndex>,
) {
    for (parent, mut text) in &mut text_query {
        let new_text = zindex_query
            .get(parent.get())
            .map_or("No ZIndex".to_string(), |zindex| match zindex {
                ZIndex::Local(value) => format!("Local({value})"),
                ZIndex::Global(value) => format!("Global({value})"),
            });

        if text.sections[0].value != new_text {
            text.sections[0].value = new_text;
        }
    }
}

#[derive(Component, Copy, Clone, Debug)]
pub struct Grab;

#[derive(Component, Copy, Clone, Debug)]
pub struct Grabbed {
    pub cursor_position: Vec2,
    pub cursor_offset: Vec2,
}

fn grab(
    mut commands: Commands,
    query: Query<(Entity, &Interaction, Option<&Grabbed>), With<Grab>>,
    windows: Res<Windows>,
) {
    for (entity, interaction, grabbed) in query.iter() {
        match interaction {
            Interaction::Clicked => {
                if grabbed.is_none() {
                    if let Some(cursor_position) = windows
                        .get_primary()
                        .and_then(|window| window.cursor_position())
                    {
                        commands.entity(entity).insert(Grabbed {
                            cursor_position,
                            cursor_offset: Vec2::new(0.0, 0.0),
                        });
                    }
                }
            }
            _ => {
                if grabbed.is_some() {
                    commands.entity(entity).remove::<Grabbed>();
                }
            }
        };
    }
}

fn grabbed_move(mut query: Query<(&mut Grabbed, &mut Style), With<Grab>>, windows: Res<Windows>) {
    for (mut grabbed, mut style) in query.iter_mut() {
        if let Some(cursor_position) = windows
            .get_primary()
            .and_then(|window| window.cursor_position())
        {
            let offset = cursor_position - grabbed.cursor_position;
            if grabbed.cursor_offset != offset {
                style.position.left += offset.x - grabbed.cursor_offset.x;
                style.position.bottom += offset.y - grabbed.cursor_offset.y;

                grabbed.cursor_offset = offset;
            }
        }
    }
}
