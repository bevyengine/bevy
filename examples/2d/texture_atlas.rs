use bevy::{asset::LoadState, prelude::*, sprite::TextureAtlasBuilder};

/// In this example we generate a new texture atlas (sprite sheet) from a folder containing individual sprites
fn main() {
    App::build()
        .init_resource::<RpgSpriteHandles>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(load_atlas.system())
        .run();
}

#[derive(Default)]
pub struct RpgSpriteHandles {
    handles: Vec<HandleUntyped>,
    atlas_loaded: bool,
}

fn setup(mut rpg_sprite_handles: ResMut<RpgSpriteHandles>, asset_server: Res<AssetServer>) {
    rpg_sprite_handles.handles = asset_server.load_folder("textures/rpg").unwrap();
}

fn load_atlas(
    mut commands: Commands,
    mut rpg_sprite_handles: ResMut<RpgSpriteHandles>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if rpg_sprite_handles.atlas_loaded {
        return;
    }

    let mut texture_atlas_builder = TextureAtlasBuilder::default();
    if let LoadState::Loaded =
        asset_server.get_group_load_state(rpg_sprite_handles.handles.iter().map(|handle| handle.id))
    {
        for handle in rpg_sprite_handles.handles.iter() {
            let texture = textures.get(handle).unwrap();
            texture_atlas_builder.add_texture(handle.clone_weak().typed::<Texture>(), &texture);
        }

        let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
        let texture_atlas_texture = texture_atlas.texture.clone();
        let vendor_handle =
            asset_server.get_handle("textures/rpg/chars/vendor/generic-rpg-vendor.png");
        let vendor_index = texture_atlas.get_texture_index(&vendor_handle).unwrap();
        let atlas_handle = texture_atlases.add(texture_atlas);

        // set up a scene to display our texture atlas
        commands
            .spawn(Camera2dComponents::default())
            // draw a sprite from the atlas
            .spawn(SpriteSheetComponents {
                transform: Transform {
                    translation: Vec3::new(150.0, 0.0, 0.0),
                    scale: Vec3::splat(4.0),
                    ..Default::default()
                },
                sprite: TextureAtlasSprite::new(vendor_index as u32),
                texture_atlas: atlas_handle,
                ..Default::default()
            })
            // draw the atlas itself
            .spawn(SpriteComponents {
                material: materials.add(texture_atlas_texture.into()),
                transform: Transform::from_translation(Vec3::new(-300.0, 0.0, 0.0)),
                ..Default::default()
            });

        rpg_sprite_handles.atlas_loaded = true;
    }
}
