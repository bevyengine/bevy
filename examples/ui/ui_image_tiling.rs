//! This example demonstrates how to use different tiling modes for a `UiImage`.

use bevy::prelude::*;
use bevy_internal::render::render_resource::{AddressMode, SamplerDescriptor};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                // This is needed for tiling textures. If you want to add tiled textures to your project, don't forget this!
                .set(ImagePlugin {
                    default_sampler: SamplerDescriptor {
                        address_mode_u: AddressMode::Repeat,
                        address_mode_v: AddressMode::Repeat,
                        address_mode_w: AddressMode::Repeat,
                        ..default()
                    },
                }),
        )
        .add_systems(Startup, setup)
        .run();
}

fn create_node_bundle(x: f32, y: f32, width: f32, height: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(width),
            height: Val::Px(height),
            ..default()
        },
        background_color: Color::rgba(1., 0., 1., 0.5).into(),
        ..default()
    }
}

fn create_text_bundle(text: &str, font: &Handle<Font>) -> TextBundle {
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 16.0,
        color: Color::BLACK,
    };
    let style = Style {
        position_type: PositionType::Absolute,
        top: Val::Px(-25.),
        left: Val::Px(0.),
        ..default()
    };
    TextBundle::from_section(text, text_style).with_style(style)
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let logo: Handle<Image> = asset_server.load("branding/icon_small.png"); // 64x64
    let font: Handle<Font> = asset_server.load("fonts/FiraMono-Medium.ttf");

    // UI camera
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(create_node_bundle(50., 50., 300., 150.))
        .with_children(|parent| {
            parent.spawn(create_text_bundle("#1 ImageBundle", &font));
            // No tiling using the ImageBundle uses the intrinsic size of the image to calculate
            //  its size, trying to maintain its aspect ratio?
            parent.spawn(ImageBundle {
                style: Style {
                    width: Val::Px(300.),
                    height: Val::Px(150.),
                    ..default()
                },
                image: UiImage {
                    texture: logo.clone(),
                    ..default()
                },
                ..default()
            });
        });

    commands
        .spawn(create_node_bundle(50., 250., 300., 150.))
        .with_children(|parent| {
            parent.spawn(create_text_bundle("#2 TilingImageBundle / None", &font));
            // No tiling (the default) using the TilingImageBundle will just stretch the image to fill the space
            parent.spawn(TiledImageBundle {
                style: Style {
                    width: Val::Px(300.),
                    height: Val::Px(150.),
                    ..default()
                },
                image: UiImage {
                    texture: logo.clone(),
                    ..default()
                },
                tiling_mode: TilingMode::None,
                ..default()
            });
        });

    commands
        .spawn(create_node_bundle(400., 50., 300., 150.))
        .with_children(|parent| {
            parent.spawn(create_text_bundle("#3 TilingImageBundle / Both", &font));
            // Tiles the image both horizontally and vertically, maintaining its original size
            parent.spawn(TiledImageBundle {
                style: Style {
                    width: Val::Px(300.),
                    height: Val::Px(150.),
                    ..default()
                },
                image: UiImage {
                    texture: logo.clone(),
                    ..default()
                },
                tiling_mode: TilingMode::Both,
                ..default()
            });
        });

    commands
        .spawn(create_node_bundle(400., 250., 300., 150.))
        .with_children(|parent| {
            parent.spawn(create_text_bundle(
                "#4 TilingImageBundle / Horizontal",
                &font,
            ));
            // Tiles the image horizontally but stretching vertically to fill the space
            parent.spawn(TiledImageBundle {
                style: Style {
                    width: Val::Px(300.),
                    height: Val::Px(150.),
                    ..default()
                },
                image: UiImage {
                    texture: logo.clone(),
                    ..default()
                },
                tiling_mode: TilingMode::Horizontal,
                ..default()
            });
        });

    commands
        .spawn(create_node_bundle(750., 250., 300., 150.))
        .with_children(|parent| {
            parent.spawn(create_text_bundle("#5 TilingImageBundle / Vertical", &font));
            // Tiles the image vertically but stretching horizontally to fill the space
            parent.spawn(TiledImageBundle {
                style: Style {
                    width: Val::Px(300.),
                    height: Val::Px(150.),
                    ..default()
                },
                image: UiImage {
                    texture: logo.clone(),
                    ..default()
                },
                tiling_mode: TilingMode::Vertical,
                ..default()
            });
        });

    // Flexbox that contains a row with two fixed sized images and
    //   one in the middle that fills the remaining space
    commands
        .spawn(create_node_bundle(50., 450., 300., 150.))
        .with_children(|parent| {
            parent.spawn(create_text_bundle("#6 Flexbox", &font));
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        width: Val::Percent(100.),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TiledImageBundle {
                        style: Style {
                            flex_grow: 0.,
                            flex_shrink: 1.,
                            width: Val::Px(64.),
                            ..default()
                        },
                        image: UiImage {
                            texture: logo.clone(),
                            ..default()
                        },
                        tiling_mode: TilingMode::Vertical,
                        ..default()
                    });
                    parent.spawn(TiledImageBundle {
                        style: Style {
                            flex_grow: 1.,
                            flex_shrink: 0.,
                            ..default()
                        },
                        image: UiImage {
                            texture: logo.clone(),
                            ..default()
                        },
                        ..default()
                    });
                    parent.spawn(TiledImageBundle {
                        style: Style {
                            flex_grow: 0.,
                            flex_shrink: 1.,
                            width: Val::Px(64.),
                            ..default()
                        },
                        image: UiImage {
                            texture: logo.clone(),
                            ..default()
                        },
                        tiling_mode: TilingMode::Vertical,
                        ..default()
                    });
                });
        });

    // Similar to the first flexbox example except now with three of such rows where
    // the middle one stretches vertically to fill the remaining space.
    // There seems to be an issue in bevy_ui where different window scaling factors
    // might cause a 1px vertical gap between the second and third row.
    commands
        .spawn(create_node_bundle(400., 450., 600., 250.))
        .with_children(|parent| {
            parent.spawn(create_text_bundle("#7 Flexbox", &font));
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                flex_grow: 0.,
                                flex_shrink: 1.,
                                height: Val::Px(64.),
                                ..default()
                            },
                            background_color: Color::YELLOW.into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 0.,
                                    flex_shrink: 1.,
                                    width: Val::Px(64.),
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::None,
                                ..default()
                            });
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 1.,
                                    flex_shrink: 0.,
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::Vertical,
                                ..default()
                            });
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 0.,
                                    flex_shrink: 1.,
                                    width: Val::Px(64.),
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::None,
                                ..default()
                            });
                        });
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                flex_grow: 1.,
                                flex_shrink: 0.,
                                ..default()
                            },
                            background_color: Color::GREEN.into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 0.,
                                    flex_shrink: 1.,
                                    width: Val::Px(64.),
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::None,
                                ..default()
                            });
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 1.,
                                    flex_shrink: 0.,
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::None,
                                ..default()
                            });
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 0.,
                                    flex_shrink: 1.,
                                    width: Val::Px(64.),
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::None,
                                ..default()
                            });
                        });
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                flex_grow: 0.,
                                flex_shrink: 1.,
                                height: Val::Px(64.),
                                ..default()
                            },
                            background_color: Color::YELLOW.into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 0.,
                                    flex_shrink: 1.,
                                    width: Val::Px(64.),
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::None,
                                ..default()
                            });
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 1.,
                                    flex_shrink: 0.,
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::Vertical,
                                ..default()
                            });
                            parent.spawn(TiledImageBundle {
                                style: Style {
                                    flex_grow: 0.,
                                    flex_shrink: 1.,
                                    width: Val::Px(64.),
                                    ..default()
                                },
                                image: UiImage {
                                    texture: logo.clone(),
                                    ..default()
                                },
                                tiling_mode: TilingMode::None,
                                ..default()
                            });
                        });
                });
        });
}
