//! Simple example demonstrating the `OverflowClipMargin` style property.

use bevy::{color::palettes::css::*, prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load("branding/icon.png");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(40.),
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
                        column_gap: Val::Px(20.),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn((
                                Node {
                                    padding: UiRect::all(Val::Px(10.)),
                                    margin: UiRect::bottom(Val::Px(25.)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                            ))
                            .with_child(Text(format!("{overflow_clip_margin:#?}")));

                        parent
                            .spawn((
                                Node {
                                    margin: UiRect::top(Val::Px(10.)),
                                    width: Val::Px(100.),
                                    height: Val::Px(100.),
                                    padding: UiRect::all(Val::Px(20.)),
                                    border: UiRect::all(Val::Px(5.)),
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
                                            min_width: Val::Px(50.),
                                            min_height: Val::Px(50.),
                                            ..default()
                                        },
                                        BackgroundColor(LIGHT_CYAN.into()),
                                    ))
                                    .with_child((
                                        ImageNode::new(image.clone()),
                                        Node {
                                            min_width: Val::Px(100.),
                                            min_height: Val::Px(100.),
                                            ..default()
                                        },
                                    ));
                            });
                    });
            }
        });
}
