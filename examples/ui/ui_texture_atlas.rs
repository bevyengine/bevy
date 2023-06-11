//! This example illustrates how to use TextureAtlases within ui

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
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((TextureAtlasImageBundle {
                style: Style {
                    width: Val::Px(256.),
                    height: Val::Px(256.),
                    border: UiRect::all(Val::Px(2.)),
                    ..default()
                },
                background_color: Color::rgb(0.65, 0.65, 0.65).into(),
                texture_atlas: texture_atlas_handle,
                texture_atlas_sprite: UiTextureAtlasSprite(TextureAtlasSprite::new(0)),
                ..default()
            },));
        });
}

fn increment_atlas_index(
    mut sprites: Query<&mut UiTextureAtlasSprite>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        for mut sprite in &mut sprites {
            sprite.index += 1;
            if sprite.index > 6 {
                sprite.index = 0;
            }
        }
    }
}
