//! This example illustrates how to use `TextureAtlases` within ui

use bevy::{prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            // This sets image filtering to nearest
            // This is done to prevent textures with low resolution (e.g. pixel art) from being blurred
            // by linear filtering.
            ImagePlugin::default_nearest(),
        ))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, increment_atlas_index)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        color: Color::ANTIQUE_WHITE,
        font_size: 20.,
        ..default()
    };

    let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(text_style.font_size * 2.),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(AtlasImageBundle {
                style: Style {
                    width: Val::Px(256.),
                    height: Val::Px(256.),
                    ..default()
                },
                texture_atlas: texture_atlas_handle,
                texture_atlas_image: UiTextureAtlasImage::default(),
                ..default()
            });
            parent.spawn(TextBundle::from_sections([
                TextSection::new("press ".to_string(), text_style.clone()),
                TextSection::new(
                    "space".to_string(),
                    TextStyle {
                        color: Color::YELLOW,
                        ..text_style.clone()
                    },
                ),
                TextSection::new(" to advance frames".to_string(), text_style),
            ]));
        });
}

fn increment_atlas_index(
    mut atlas_images: Query<&mut UiTextureAtlasImage>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        for mut atlas_image in &mut atlas_images {
            atlas_image.index = (atlas_image.index + 1) % 6;
        }
    }
}
