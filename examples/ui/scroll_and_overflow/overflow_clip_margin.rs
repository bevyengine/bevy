//! Simple example demonstrating the `OverflowClipMargin` style property.

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load("branding/icon.png");

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(40),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(ANTIQUE_WHITE.into()),
        ))
        .with_children(|parent| {
            for overflow_clip_margin in [
                OverflowClipMargin::border_box().with_margin(25.),
                OverflowClipMargin::border_box(),
                OverflowClipMargin::padding_box(),
                OverflowClipMargin::content_box(),
            ] {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(20),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn((
                                Node {
                                    padding: UiRect::all(px(10)),
                                    margin: UiRect::bottom(px(25)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                            ))
                            .with_child(Text(format!("{overflow_clip_margin:#?}")));

                        parent
                            .spawn((
                                Node {
                                    margin: UiRect::top(px(10)),
                                    width: px(100),
                                    height: px(100),
                                    padding: UiRect::all(px(20)),
                                    border: UiRect::all(px(5)),
                                    overflow: Overflow::clip(),
                                    overflow_clip_margin,
                                    ..default()
                                },
                                BackgroundColor(GRAY.into()),
                                BorderColor::all(Color::BLACK),
                            ))
                            .with_children(|parent| {
                                parent
                                    .spawn((
                                        Node {
                                            min_width: px(50),
                                            min_height: px(50),
                                            ..default()
                                        },
                                        BackgroundColor(LIGHT_CYAN.into()),
                                    ))
                                    .with_child((
                                        ImageNode::new(image.clone()),
                                        Node {
                                            min_width: px(100),
                                            min_height: px(100),
                                            ..default()
                                        },
                                    ));
                            });
                    });
            }
        });
}
