//! Demonstrates multiple windows each with their own UI layout

use bevy::{prelude::*, render::camera::RenderTarget, window::WindowRef};
use bevy_internal::ui::UiTargetCamera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                bevy::window::close_on_esc,
                update_buttons::<FirstWindowNode, SecondWindowNode>,
                update_buttons::<SecondWindowNode, FirstWindowNode>,
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

fn setup_scene(mut commands: Commands) {
    // Primary window camera
    commands.spawn(Camera3dBundle::default());

    // Spawn a second window
    let second_window = commands
        .spawn(Window {
            title: "Second Window".to_owned(),
            ..default()
        })
        .id();

    // Secondary window camera
    let second_camera = commands
        .spawn(Camera3dBundle {
            camera: Camera {
                target: RenderTarget::Window(WindowRef::Entity(second_window)),
                ..default()
            },
            ..default()
        })
        .id();

    spawn_nodes::<FirstWindowNode>(&mut commands, "first window", None);

    spawn_nodes::<SecondWindowNode>(&mut commands, "second window", Some(second_camera));
}

fn spawn_nodes<M: Component + Default>(
    commands: &mut Commands,
    title: &str,
    camera_target: Option<Entity>,
) {
    let mut ec = commands.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            column_gap: Val::Px(30.),
            ..Default::default()
        },
        ..Default::default()
    });
    ec.with_children(|builder| {
        builder.spawn(TextBundle::from_section(
            title,
            TextStyle {
                font_size: 50.,
                ..Default::default()
            },
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
            M::default(),
        ));

        builder
            .spawn((
                ButtonBundle {
                    button: Button,
                    style: Style {
                        padding: UiRect::all(Val::Px(10.)),
                        ..Default::default()
                    },
                    background_color: Color::BLACK.into(),
                    ..Default::default()
                },
                M::default(),
            ))
            .with_children(|builder| {
                builder.spawn(TextBundle::from_section(
                    format!("{title} button"),
                    TextStyle {
                        font_size: 50.,
                        ..Default::default()
                    },
                ));
            });
    });

    if let Some(view) = camera_target {
        ec.insert(UiTargetCamera { entity: view });
    }
}

fn update_buttons<M: Component + Default, N: Component + Default>(
    mut button_query: Query<(Ref<Interaction>, &mut BackgroundColor), With<M>>,
    mut text_query: Query<(&mut Value, &mut Text), With<N>>,
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
                    color.0 = Color::NAVY;
                }
                Interaction::None => {
                    color.0 = Color::BLACK;
                }
            }
        }
    }
}
