//! In this example we generate 4 texture atlases (sprite sheets) from a folder containing
//! individual sprites.
//!
//! The texture atlases are generated with different padding and sampling to demonstrate the
//! effect of these settings, and how bleeding issues can be resolved by padding the sprites.
//!
//! Only one padded and one unpadded texture atlas are rendered to the screen.
//! An upscaled sprite from each of the 4 atlases are rendered to the screen.

use bevy::{asset::LoadedFolder, prelude::*, render::texture::ImageSampler};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // fallback to nearest sampling
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
    // and that the the font has been loaded by the `FontSystem`.
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
    let loaded_folder = loaded_folders.get(&rpg_sprite_handles.0).unwrap();

    // create texture atlases with different padding and sampling

    let texture_atlas_linear = create_texture_atlas(
        loaded_folder,
        None,
        Some(ImageSampler::linear()),
        &mut textures,
    );
    let atlas_linear_handle = texture_atlases.add(texture_atlas_linear.clone());

    let texture_atlas_nearest = create_texture_atlas(
        loaded_folder,
        None,
        Some(ImageSampler::nearest()),
        &mut textures,
    );
    let atlas_nearest_handle = texture_atlases.add(texture_atlas_nearest.clone());

    let texture_atlas_linear_padded = create_texture_atlas(
        loaded_folder,
        Some(UVec2::new(6, 6)),
        Some(ImageSampler::linear()),
        &mut textures,
    );
    let atlas_linear_padded_handle = texture_atlases.add(texture_atlas_linear_padded.clone());

    let texture_atlas_nearest_padded = create_texture_atlas(
        loaded_folder,
        Some(UVec2::new(6, 6)),
        Some(ImageSampler::nearest()),
        &mut textures,
    );
    let atlas_nearest_padded_handle = texture_atlases.add(texture_atlas_nearest_padded.clone());

    // setup 2d scene
    commands.spawn(Camera2dBundle::default());

    // padded textures are to the right, unpadded to the left

    // draw unpadded texture atlas
    commands.spawn(SpriteBundle {
        texture: texture_atlas_linear_padded.texture.clone(),
        transform: Transform {
            translation: Vec3::new(-250.0, -130.0, 0.0),
            scale: Vec3::splat(0.8),
            ..default()
        },
        ..default()
    });

    // draw padded texture atlas
    commands.spawn(SpriteBundle {
        texture: texture_atlas_linear_padded.texture.clone(),
        transform: Transform {
            translation: Vec3::new(250.0, -130.0, 0.0),
            scale: Vec3::splat(0.8),
            ..default()
        },
        ..default()
    });

    // draw sprites from texture atlases

    // get handle to a sprite to render
    let vendor_handle: Handle<Image> = asset_server
        .get_handle("textures/rpg/chars/vendor/generic-rpg-vendor.png")
        .unwrap();

    // linear, no padding
    commands.spawn(SpriteSheetBundle {
        transform: Transform {
            translation: Vec3::new(-350.0, 170.0, 0.0),
            scale: Vec3::splat(3.0),
            ..default()
        },
        sprite: TextureAtlasSprite::new(
            texture_atlas_linear
                .get_texture_index(&vendor_handle)
                .unwrap(),
        ),
        texture_atlas: atlas_linear_handle,
        ..default()
    });

    // nearest, no padding
    commands.spawn(SpriteSheetBundle {
        transform: Transform {
            translation: Vec3::new(-150.0, 170.0, 0.0),
            scale: Vec3::splat(3.0),
            ..default()
        },
        sprite: TextureAtlasSprite::new(
            texture_atlas_nearest
                .get_texture_index(&vendor_handle)
                .unwrap(),
        ),
        texture_atlas: atlas_nearest_handle,
        ..default()
    });

    // linear, padding
    commands.spawn(SpriteSheetBundle {
        transform: Transform {
            translation: Vec3::new(150.0, 170.0, 0.0),
            scale: Vec3::splat(3.0),
            ..default()
        },
        sprite: TextureAtlasSprite::new(
            texture_atlas_linear_padded
                .get_texture_index(&vendor_handle)
                .unwrap(),
        ),
        texture_atlas: atlas_linear_padded_handle,
        ..default()
    });

    // nearest, padding
    commands.spawn(SpriteSheetBundle {
        transform: Transform {
            translation: Vec3::new(350.0, 170.0, 0.0),
            scale: Vec3::splat(3.0),
            ..default()
        },
        sprite: TextureAtlasSprite::new(
            texture_atlas_nearest_padded
                .get_texture_index(&vendor_handle)
                .unwrap(),
        ),
        texture_atlas: atlas_nearest_padded_handle,
        ..default()
    });
}

/// Create a texture atlas with the given padding and sampling settings
/// from the individual sprites in the given folder.
fn create_texture_atlas(
    folder: &LoadedFolder,
    padding: Option<UVec2>,
    sampling: Option<ImageSampler>,
    textures: &mut ResMut<Assets<Image>>,
) -> TextureAtlas {
    // Build a `TextureAtlas` using the individual sprites
    let mut texture_atlas_builder =
        TextureAtlasBuilder::default().padding(padding.unwrap_or_default());
    for handle in folder.handles.iter() {
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

    let texture_atlas = texture_atlas_builder.finish(textures).unwrap();

    // Update the sampling settings of the texture atlas
    let image = textures.get_mut(&texture_atlas.texture).unwrap();
    image.sampler_descriptor = sampling.unwrap_or_default();

    texture_atlas
}
