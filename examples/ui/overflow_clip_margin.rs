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
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(40.),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background_color: ANTIQUE_WHITE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            for overflow_clip_margin in [
                OverflowClipMargin::border_box().with_margin(25.),
                OverflowClipMargin::border_box(),
                OverflowClipMargin::padding_box(),
                OverflowClipMargin::content_box(),
            ] {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    padding: UiRect::all(Val::Px(10.)),
                                    margin: UiRect::bottom(Val::Px(25.)),
                                    ..Default::default()
                                },
                                background_color: Color::srgb(0.25, 0.25, 0.25).into(),
                                ..Default::default()
                            })
                            .with_child(Text(format!("{overflow_clip_margin:#?}")));

                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    margin: UiRect::top(Val::Px(10.)),
                                    width: Val::Px(100.),
                                    height: Val::Px(100.),
                                    padding: UiRect::all(Val::Px(20.)),
                                    border: UiRect::all(Val::Px(5.)),
                                    overflow: Overflow::clip(),
                                    overflow_clip_margin,
                                    ..Default::default()
                                },
                                border_color: Color::BLACK.into(),
                                background_color: GRAY.into(),
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent
                                    .spawn(NodeBundle {
                                        style: Style {
                                            min_width: Val::Px(50.),
                                            min_height: Val::Px(50.),
                                            ..Default::default()
                                        },
                                        background_color: LIGHT_CYAN.into(),
                                        ..Default::default()
                                    })
                                    .with_child(ImageBundle {
                                        image: UiImage::new(image.clone()),
                                        style: Style {
                                            min_width: Val::Px(100.),
                                            min_height: Val::Px(100.),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    });
                            });
                    });
            }
        });
}
