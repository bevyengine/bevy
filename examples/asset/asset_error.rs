use bevy::prelude::*;
use bevy::asset::{AssetServerError, AssetIoError};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(change_failed_textures.system())
        .run();
}

struct MySpecialSprite {
    handle: Handle<Texture>
}

/// What to use if the sprite errored
struct FallbackMaterials {
    /// if the sprite does not exist
    missing: Handle<ColorMaterial>,
    /// if the file exists but failed to load
    load_error: Handle<ColorMaterial>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // This file does not exist in the bevy repo.
    // If you add it, the example will just use it, but otherwise
    // it will be replaced with our fallback material.
    let texture_handle = asset_server.load("awesome_sprite.png");

    // store the handle in a resource to track it later
    commands.insert_resource(MySpecialSprite {
        handle: texture_handle.clone()
    });

    // prepare our fallbacks
    commands.insert_resource(FallbackMaterials {
        // solid pink color for missing
        missing: materials.add(Color::PINK.into()),
        // solid red color for other errors
        load_error: materials.add(Color::RED.into()),
    });

    commands
        .spawn(OrthographicCameraBundle::new_2d())
        .spawn(SpriteBundle {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}

fn change_failed_textures(
    asset_server: Res<AssetServer>,
    my_sprite: Res<MySpecialSprite>,
    fallbacks: Res<FallbackMaterials>,
    mut mat_q: Query<(Option<&mut Sprite>, &mut Handle<ColorMaterial>)>,
) {
    // The error is only returned once, so we can call this repeatedly
    if let Some(error) = asset_server.get_error(&my_sprite.handle) {
        // choose our fallback depending on the error value
        let fallback = match error {
            AssetServerError::AssetIoError(AssetIoError::NotFound(_)) => fallbacks.missing.clone(),
            _ => fallbacks.load_error.clone(),
        };

        // replace all existing uses
        for (sprite, mut material) in mat_q.iter_mut() {
            *material = fallback.clone();

            if let Some(mut sprite) = sprite {
                *sprite = Sprite::new(Vec2::splat(16.0));
            }
        }
    }
}
