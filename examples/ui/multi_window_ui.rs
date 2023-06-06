//! Demonstrates multiple windows each with their own UI layout

use bevy::{prelude::*, render::camera::RenderTarget, window::WindowRef};
use bevy_internal::{
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    ui::UiTargetCamera,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                bevy::window::close_on_esc,
                update::<FirstWindowNode, SecondWindowNode>,
                update::<SecondWindowNode, FirstWindowNode>,
            ),
        )
        .run();
}

#[derive(Component, Default)]
struct FirstWindowNode;

#[derive(Component, Default)]
struct SecondWindowNode;

#[derive(Component, Default)]
struct Value(u64);

fn setup_scene(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Primary window camera
    commands.spawn(Camera2dBundle::default());

    // Spawn a second window
    let second_window = commands
        .spawn(Window {
            title: "Second Window".to_owned(),
            ..default()
        })
        .id();

    // Secondary window camera
    let second_camera = commands
        .spawn(Camera2dBundle {
            camera: Camera {
                target: RenderTarget::Window(WindowRef::Entity(second_window)),
                ..default()
            },
            ..default()
        })
        .id();

    spawn_nodes::<FirstWindowNode>(&mut commands, "first window", None);

    spawn_nodes::<SecondWindowNode>(&mut commands, "second window", Some(second_camera));

    let size = Extent3d {
        width: 200,
        height: 300,
        ..default()
    };
    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);

    commands.spawn(SpriteBundle {
        texture: image_handle.clone(),
        ..Default::default()
    });

    let sprite_camera = commands
        .spawn(Camera2dBundle {
            camera: Camera {
                target: RenderTarget::Image(image_handle),
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    border: UiRect::all(Val::Px(10.)),
                    ..Default::default()
                },
                background_color: Color::WHITE.into(),
                ..Default::default()
            },
            UiTargetCamera {
                entity: sprite_camera,
            },
        ))
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                    background_color: Color::MAROON.into(),
                    ..Default::default()
                })
                .with_children(|builder| {
                    builder.spawn(
                        TextBundle::from_section(
                            "UI rendered\nto\na sprite",
                            TextStyle {
                                font_size: 25.,
                                ..Default::default()
                            },
                        )
                        .with_text_alignment(TextAlignment::Center),
                    );
                    builder
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..Default::default()
                                },
                                background_color: Color::WHITE.into(),
                                ..Default::default()
                            },
                            ExitButton,
                        ))
                        .with_children(|builder| {
                            builder.spawn(
                                TextBundle::from_section(
                                    "EXIT",
                                    TextStyle {
                                        font_size: 35.,
                                        color: Color::BLACK,
                                        ..Default::default()
                                    },
                                )
                                .with_style(Style {
                                    margin: UiRect::all(Val::Px(10.)),
                                    ..Default::default()
                                }),
                            );
                        });
                    builder.spawn((
                        TextBundle::from_section(
                            "0",
                            TextStyle {
                                font_size: 50.,
                                ..Default::default()
                            },
                        ),
                        Value(0),
                        FirstWindowNode,
                    ));
                    builder.spawn((
                        TextBundle::from_section(
                            "0",
                            TextStyle {
                                font_size: 50.,
                                ..Default::default()
                            },
                        ),
                        Value(0),
                        SecondWindowNode,
                    ));
                });
        });
}

#[derive(Component)]
struct ExitButton;

fn spawn_nodes<M: Component + Default>(
    commands: &mut Commands,
    title: &str,
    camera_target: Option<Entity>,
) {
    let mut entity_commands = commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        ..Default::default()
    });

    if let Some(view) = camera_target {
        entity_commands.insert(UiTargetCamera { entity: view });
    }

    entity_commands.with_children(|builder| {
        builder
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(50.),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::all(Val::Vw(5.)),
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|builder| {
                builder.spawn(TextBundle::from_section(
                    title,
                    TextStyle {
                        font_size: 50.,
                        ..Default::default()
                    },
                ));

                builder
                    .spawn((
                        ButtonBundle {
                            button: Button,
                            style: Style {
                                padding: UiRect::all(Val::Px(10.)),
                                ..Default::default()
                            },
                            background_color: Color::WHITE.into(),
                            ..Default::default()
                        },
                        M::default(),
                    ))
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section(
                            format!("{title} button"),
                            TextStyle {
                                font_size: 35.,
                                color: Color::BLACK,
                                ..Default::default()
                            },
                        ));
                    });
            });
    });
}

fn update<M: Component + Default, N: Component + Default>(
    time: Res<Time>,
    mut button_query: Query<(Ref<Interaction>, &mut BackgroundColor), With<M>>,
    mut text_query: Query<(&mut Value, &mut Text), With<N>>,
    mut sprite_query: Query<&mut Transform, With<Sprite>>,
) {
    for (interaction, mut color) in button_query.iter_mut() {
        if interaction.is_changed() {
            match *interaction {
                Interaction::Clicked => {
                    for (mut value, mut text) in text_query.iter_mut() {
                        value.0 += 1;
                        text.sections[0].value = format!("{}", value.0);
                    }
                    color.0 = Color::RED;
                }
                Interaction::Hovered => {
                    color.0 = Color::YELLOW;
                }
                Interaction::None => {
                    color.0 = Color::WHITE;
                }
            }
        }
    }

    for mut transform in sprite_query.iter_mut() {
        transform.rotation = Quat::from_rotation_z(0.25 * (0.25 * time.elapsed_seconds()).sin())
    }
}
