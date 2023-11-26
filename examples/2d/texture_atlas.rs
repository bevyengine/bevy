//! In this example we generate a new texture atlas (sprite sheet) from a folder containing
//! individual sprites.

use bevy::{asset::LoadedFolder, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_state::<AppState>()
        .add_systems(OnEnter(AppState::Setup), load_textures)
        .add_systems(Update, check_textures.run_if(in_state(AppState::Setup)))
        .add_systems(OnEnter(AppState::Finished), setup)
        .run();
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
enum AppState {
    #[default]
    Setup,
    Finished,
}

#[derive(Resource, Default)]
struct RpgSpriteFolder(Handle<LoadedFolder>);

fn load_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    // load multiple, individual sprites from a folder
    commands.insert_resource(RpgSpriteFolder(asset_server.load_folder("textures/rpg")));
}

fn check_textures(
    mut next_state: ResMut<NextState<AppState>>,
    rpg_sprite_folder: ResMut<RpgSpriteFolder>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
) {
    // Advance the `AppState` once all sprite handles have been loaded by the `AssetServer`
    for event in events.read() {
        if event.is_loaded_with_dependencies(&rpg_sprite_folder.0) {
            next_state.set(AppState::Finished);
        }
    }
}

fn setup(
    mut commands: Commands,
    rpg_sprite_handles: Res<RpgSpriteFolder>,
    asset_server: Res<AssetServer>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Image>>,
) {
    // Build a `TextureAtlas` using the individual sprites
    let mut texture_atlas_builder = TextureAtlasBuilder::default();
    let loaded_folder = loaded_folders.get(&rpg_sprite_handles.0).unwrap();
    for handle in loaded_folder.handles.iter() {
        let id = handle.id().typed_unchecked::<Image>();
        let Some(texture) = textures.get(id) else {
            warn!(
                "{:?} did not resolve to an `Image` asset.",
                handle.path().unwrap()
            );
            continue;
        };

        texture_atlas_builder.add_texture(id, texture);
    }

    let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
    let texture_atlas_texture = texture_atlas.texture.clone();
    let vendor_handle = asset_server
        .get_handle("textures/rpg/chars/vendor/generic-rpg-vendor.png")
        .unwrap();
    let vendor_index = texture_atlas.get_texture_index(&vendor_handle).unwrap();
    let atlas_handle = texture_atlases.add(texture_atlas);

    // set up a scene to display our texture atlas
    commands.spawn(Camera2dBundle::default());
    // draw a sprite from the atlas
    commands.spawn(SpriteSheetBundle {
        transform: Transform {
            translation: Vec3::new(150.0, 0.0, 0.0),
            scale: Vec3::splat(4.0),
            ..default()
        },
        sprite: TextureAtlasSprite::new(vendor_index),
        texture_atlas: atlas_handle,
        ..default()
    });
    // draw the atlas itself
    commands.spawn(SpriteBundle {
        texture: texture_atlas_texture,
        transform: Transform::from_xyz(-300.0, 0.0, 0.0),
        ..default()
    });
}
