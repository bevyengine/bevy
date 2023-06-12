//! Simple example demonstrating overflow behavior.

use bevy::{prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 20.0,
        color: Color::WHITE,
    };

    let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                width: Val::Percent(100.),
                ..Default::default()
            },
            background_color: Color::ANTIQUE_WHITE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            for overflow in [
                Overflow::visible(),
                Overflow::clip_x(),
                Overflow::clip_y(),
                Overflow::clip(),
            ] {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            margin: UiRect::horizontal(Val::Px(25.)),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        let label = format!("{overflow:#?}");
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    padding: UiRect::all(Val::Px(10.)),
                                    margin: UiRect::bottom(Val::Px(25.)),
                                    ..Default::default()
                                },
                                background_color: Color::DARK_GRAY.into(),
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent.spawn(TextBundle {
                                    text: Text::from_section(label, text_style.clone()),
                                    ..Default::default()
                                });
                            });
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    width: Val::Px(100.),
                                    height: Val::Px(100.),
                                    padding: UiRect {
                                        left: Val::Px(25.),
                                        top: Val::Px(25.),
                                        ..Default::default()
                                    },
                                    overflow,
                                    ..Default::default()
                                },
                                background_color: Color::GRAY.into(),
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent.spawn(AtlasImageBundle {
                                    texture_atlas: texture_atlas_handle.clone(),
                                    texture_atlas_sprite: UiTextureAtlasSprite(
                                        TextureAtlasSprite::new(3),
                                    ),
                                    style: Style {
                                        min_width: Val::Px(100.),
                                        min_height: Val::Px(100.),
                                        ..Default::default()
                                    },
                                    background_color: Color::WHITE.into(),
                                    ..Default::default()
                                });
                            });
                    });
            }
        });
}
