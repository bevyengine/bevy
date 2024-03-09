//! A simple example for debugging viewport coordinates
//!
//! This example creates two uinode trees, one using viewport coordinates and one using pixel coordinates,
//! and then switches between them once per second using the `Display` style property.
//! If there are no problems both layouts should be identical, except for the color of the margin changing which is used to signal that the displayed uinode tree has changed
//! (red for viewport, yellow for pixel).
use bevy::{color::palettes::css::*, prelude::*};

const PALETTE: [Srgba; 10] = [
    RED, YELLOW, WHITE, BEIGE, AQUA, CRIMSON, NAVY, AZURE, LIME, BLACK,
];

#[derive(Component, Default, PartialEq)]
enum Coords {
    #[default]
    Viewport,
    Pixel,
}

fn main() {
    App::new()
        .insert_resource(UiScale(2.0))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Viewport Coordinates Debug".to_string(),
                // This example relies on these specific viewport dimensions, so let's explicitly
                // define them.
                resolution: [1280., 720.].into(),
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn update(
    mut timer: Local<f32>,
    mut visible_tree: Local<Coords>,
    time: Res<Time>,
    mut coords_style_query: Query<(&Coords, &mut Style)>,
) {
    *timer -= time.delta_seconds();
    if *timer <= 0. {
        *timer = 1.;
        *visible_tree = match *visible_tree {
            Coords::Viewport => Coords::Pixel,
            Coords::Pixel => Coords::Viewport,
        };
        for (coords, mut style) in coords_style_query.iter_mut() {
            style.display = if *coords == *visible_tree {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    spawn_with_viewport_coords(&mut commands);
    spawn_with_pixel_coords(&mut commands);
}

fn spawn_with_viewport_coords(commands: &mut Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Vw(100.),
                    height: Val::Vh(100.),
                    border: UiRect::axes(Val::Vw(5.), Val::Vh(5.)),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                border_color: PALETTE[0].into(),
                ..default()
            },
            Coords::Viewport,
        ))
        .with_children(|builder| {
            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(30.),
                    height: Val::Vh(30.),
                    border: UiRect::all(Val::VMin(5.)),
                    ..default()
                },
                background_color: PALETTE[2].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(60.),
                    height: Val::Vh(30.),
                    ..default()
                },
                background_color: PALETTE[3].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(45.),
                    height: Val::Vh(30.),
                    border: UiRect::left(Val::VMax(45. / 2.)),
                    ..default()
                },
                background_color: PALETTE[4].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(45.),
                    height: Val::Vh(30.),
                    border: UiRect::right(Val::VMax(45. / 2.)),
                    ..default()
                },
                background_color: PALETTE[5].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(60.),
                    height: Val::Vh(30.),
                    ..default()
                },
                background_color: PALETTE[6].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(30.),
                    height: Val::Vh(30.),
                    border: UiRect::all(Val::VMin(5.)),
                    ..default()
                },
                background_color: PALETTE[7].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });
        });
}

fn spawn_with_pixel_coords(commands: &mut Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(640.),
                    height: Val::Px(360.),
                    border: UiRect::axes(Val::Px(32.), Val::Px(18.)),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                border_color: PALETTE[1].into(),
                ..default()
            },
            Coords::Pixel,
        ))
        .with_children(|builder| {
            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(192.),
                    height: Val::Px(108.),
                    border: UiRect::axes(Val::Px(18.), Val::Px(18.)),
                    ..default()
                },
                background_color: PALETTE[2].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(384.),
                    height: Val::Px(108.),
                    ..default()
                },
                background_color: PALETTE[3].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(288.),
                    height: Val::Px(108.),
                    border: UiRect::left(Val::Px(144.)),
                    ..default()
                },
                background_color: PALETTE[4].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(288.),
                    height: Val::Px(108.),
                    border: UiRect::right(Val::Px(144.)),
                    ..default()
                },
                background_color: PALETTE[5].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(384.),
                    height: Val::Px(108.),
                    ..default()
                },
                background_color: PALETTE[6].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(192.),
                    height: Val::Px(108.),
                    border: UiRect::axes(Val::Px(18.), Val::Px(18.)),
                    ..default()
                },
                background_color: PALETTE[7].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });
        });
}
