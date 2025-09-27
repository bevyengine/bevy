//! Shows an "anchor layout" style of ui layout
use bevy::prelude::*;

const MARGIN: Val = Val::Px(12.);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Anchor Layout Example".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, spawn_layout)
        .run();
}

fn spawn_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                // fill the entire window
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: MARGIN.all(),
                row_gap: MARGIN,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|builder| {
            let rows = [
                [
                    (
                        "left: 10px\ntop: 10px",
                        Node {
                            left: px(10),
                            top: px(10),
                            ..default()
                        },
                    ),
                    (
                        "center: 10px\ntop: 10px",
                        Node {
                            margin: auto().horizontal(),
                            top: px(10),
                            ..default()
                        },
                    ),
                    (
                        "right: 10px\ntop: 10px",
                        Node {
                            right: px(10),
                            top: px(10),
                            ..default()
                        },
                    ),
                ],
                [
                    (
                        "left: 10px\ncenter: 10px",
                        Node {
                            left: px(10),
                            margin: UiRect::vertical(Val::Auto),
                            ..default()
                        },
                    ),
                    (
                        "center: 10px\ncenter: 10px",
                        Node {
                            margin: UiRect::all(Val::Auto),
                            ..default()
                        },
                    ),
                    (
                        "right: 10px\ncenter: 10px",
                        Node {
                            right: px(10),
                            margin: UiRect::vertical(Val::Auto),
                            ..default()
                        },
                    ),
                ],
                [
                    (
                        "left: 10px\nbottom: 10px",
                        Node {
                            left: px(10),
                            bottom: px(10),
                            ..default()
                        },
                    ),
                    (
                        "center: 10px\nbottom: 10px",
                        Node {
                            margin: UiRect::horizontal(Val::Auto),
                            bottom: px(10),
                            ..default()
                        },
                    ),
                    (
                        "right: 10px\nbottom: 10px",
                        Node {
                            right: px(10),
                            bottom: px(10),
                            ..default()
                        },
                    ),
                ],
            ];
            for row in rows {
                let font = font.clone();

                builder.spawn((
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Row,
                        column_gap: MARGIN,
                        ..default()
                    },
                    Children::spawn(SpawnIter(
                        row.into_iter()
                            .map(move |v| anchored_node(font.clone(), v.1, v.0)),
                    )),
                ));
            }
        });
}

fn anchored_node(font: Handle<Font>, node: Node, label: &str) -> impl Bundle {
    (
        // outer gray box
        Node {
            display: Display::Flex,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        children![
            // inner label box
            (
                Node {
                    display: Display::Block,
                    padding: UiRect::axes(px(5), px(1)),
                    position_type: PositionType::Absolute,
                    ..node
                },
                BackgroundColor(Color::srgb(1., 0.066, 0.349)),
                children![(
                    Text::new(label),
                    TextFont { font, ..default() },
                    TextColor::BLACK,
                )],
            )
        ],
    )
}
