//! A simple example for debugging viewport coordinates
//!
//! This example creates two UI node trees, one using viewport coordinates and one using pixel coordinates,
//! and then switches between them once per second using the `Display` style property.
//! If there are no problems both layouts should be identical, except for the color of the margin changing which is used to signal that the displayed UI node tree has changed
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
                resolution: (1280, 720).into(),
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
    mut coords_nodes: Query<(&Coords, &mut Node)>,
) {
    *timer -= time.delta_secs();
    if *timer <= 0. {
        *timer = 1.;
        *visible_tree = match *visible_tree {
            Coords::Viewport => Coords::Pixel,
            Coords::Pixel => Coords::Viewport,
        };
        for (coords, mut node) in coords_nodes.iter_mut() {
            node.display = if *coords == *visible_tree {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    spawn_with_viewport_coords(&mut commands);
    spawn_with_pixel_coords(&mut commands);
}

fn spawn_with_viewport_coords(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                width: vw(100),
                height: vh(100),
                border: UiRect::axes(vw(5), vh(5)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            BorderColor::all(PALETTE[0]),
            Coords::Viewport,
        ))
        .with_children(|builder| {
            builder.spawn((
                Node {
                    width: vw(30),
                    height: vh(30),
                    border: UiRect::all(vmin(5)),
                    ..default()
                },
                BackgroundColor(PALETTE[2].into()),
                BorderColor::all(PALETTE[9]),
            ));

            builder.spawn((
                Node {
                    width: vw(60),
                    height: vh(30),
                    ..default()
                },
                BackgroundColor(PALETTE[3].into()),
            ));

            builder.spawn((
                Node {
                    width: vw(45),
                    height: vh(30),
                    border: UiRect::left(vmax(45. / 2.)),
                    ..default()
                },
                BackgroundColor(PALETTE[4].into()),
                BorderColor::all(PALETTE[8]),
            ));

            builder.spawn((
                Node {
                    width: vw(45),
                    height: vh(30),
                    border: UiRect::right(vmax(45. / 2.)),
                    ..default()
                },
                BackgroundColor(PALETTE[5].into()),
                BorderColor::all(PALETTE[8]),
            ));

            builder.spawn((
                Node {
                    width: vw(60),
                    height: vh(30),
                    ..default()
                },
                BackgroundColor(PALETTE[6].into()),
            ));

            builder.spawn((
                Node {
                    width: vw(30),
                    height: vh(30),
                    border: UiRect::all(vmin(5)),
                    ..default()
                },
                BackgroundColor(PALETTE[7].into()),
                BorderColor::all(PALETTE[9]),
            ));
        });
}

fn spawn_with_pixel_coords(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                width: px(640),
                height: px(360),
                border: UiRect::axes(px(32), px(18)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            BorderColor::all(PALETTE[1]),
            Coords::Pixel,
        ))
        .with_children(|builder| {
            builder.spawn((
                Node {
                    width: px(192),
                    height: px(108),
                    border: UiRect::axes(px(18), px(18)),
                    ..default()
                },
                BackgroundColor(PALETTE[2].into()),
                BorderColor::all(PALETTE[9]),
            ));

            builder.spawn((
                Node {
                    width: px(384),
                    height: px(108),
                    ..default()
                },
                BackgroundColor(PALETTE[3].into()),
            ));

            builder.spawn((
                Node {
                    width: px(288),
                    height: px(108),
                    border: UiRect::left(px(144)),
                    ..default()
                },
                BackgroundColor(PALETTE[4].into()),
                BorderColor::all(PALETTE[8]),
            ));

            builder.spawn((
                Node {
                    width: px(288),
                    height: px(108),
                    border: UiRect::right(px(144)),
                    ..default()
                },
                BackgroundColor(PALETTE[5].into()),
                BorderColor::all(PALETTE[8]),
            ));

            builder.spawn((
                Node {
                    width: px(384),
                    height: px(108),
                    ..default()
                },
                BackgroundColor(PALETTE[6].into()),
            ));

            builder.spawn((
                Node {
                    width: px(192),
                    height: px(108),
                    border: UiRect::axes(px(18), px(18)),
                    ..default()
                },
                BackgroundColor(PALETTE[7].into()),
                BorderColor::all(PALETTE[9]),
            ));
        });
}
