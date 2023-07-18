//! A simple example for debugging viewport coordinates
//!
//! This example creates two uinode trees, one using viewport coordinates and one using pixel coordinates,
//! and then switches between them once per second using the `Display` style property.
//! If there are no problems both layouts should be identical, except for the color of the margin changing which is used to signal that the displayed uinode tree has changed
//! (red for viewport, yellow for pixel).
use bevy::prelude::*;

const PALETTE: [Color; 10] = [
    Color::RED,
    Color::YELLOW,
    Color::WHITE,
    Color::BEIGE,
    Color::CYAN,
    Color::CRIMSON,
    Color::NAVY,
    Color::AZURE,
    Color::GREEN,
    Color::BLACK,
];

#[derive(Component, Default, PartialEq)]
enum Coords {
    #[default]
    Viewport,
    Pixel,
}

fn main() {
    App::new()
        .insert_resource(UiScale { scale: 2.0 })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [1600., 1200.].into(),
                title: "Viewport Coordinates Debug".to_string(),
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
                    width: Val::Px(800.),
                    height: Val::Px(600.),
                    border: UiRect::axes(Val::Px(40.), Val::Px(30.)),
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
                    width: Val::Px(240.),
                    height: Val::Px(180.),
                    border: UiRect::axes(Val::Px(30.), Val::Px(30.)),
                    ..default()
                },
                background_color: PALETTE[2].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(480.),
                    height: Val::Px(180.),
                    ..default()
                },
                background_color: PALETTE[3].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(360.),
                    height: Val::Px(180.),
                    border: UiRect::left(Val::Px(180.)),
                    ..default()
                },
                background_color: PALETTE[4].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(360.),
                    height: Val::Px(180.),
                    border: UiRect::right(Val::Px(180.)),
                    ..default()
                },
                background_color: PALETTE[5].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(480.),
                    height: Val::Px(180.),
                    ..default()
                },
                background_color: PALETTE[6].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(240.),
                    height: Val::Px(180.),
                    border: UiRect::axes(Val::Px(30.), Val::Px(30.)),
                    ..default()
                },
                background_color: PALETTE[7].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });
        });
}
