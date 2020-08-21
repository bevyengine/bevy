use bevy::{asset::LoadStatus, prelude::*, sprite::TextureAtlasBuilder};

/// In this example we generate a new texture atlas (sprite sheet) from a folder containing individual sprites
fn main() {
    App::build()
        .init_resource::<RpgSpriteHandles>()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(load_atlas.system())
        .run();
}

#[derive(Default)]
pub struct RpgSpriteHandles {
    handles: Vec<Handle<Texture>>,
    atlas_loaded: bool,
}

fn setup(mut rpg_sprite_handles: ResMut<RpgSpriteHandles>, asset_server: Res<AssetServer>) {
    rpg_sprite_handles.handles = asset_server.load_folder("assets/textures/rpg").unwrap();
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
    if let Some(LoadStatus::Loaded(_)) =
        asset_server.get_group_load_status(&rpg_sprite_handles.handles)
    {
        for &handle in rpg_sprite_handles.handles.iter() {
            let texture = textures.get(&handle).unwrap();
            texture_atlas_builder.add_texture(handle, &texture);
        }

        let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
        let texture_atlas_texture = texture_atlas.texture;
        let vendor_handle = asset_server
            .get_handle("assets/textures/rpg/chars/vendor/generic-rpg-vendor.png")
            .unwrap();
        let vendor_index = texture_atlas.get_texture_index(vendor_handle).unwrap();
        let atlas_handle = texture_atlases.add(texture_atlas);

        // set up a scene to display our texture atlas
        commands
            .spawn(Camera2dComponents::default())
            // draw a sprite from the atlas
            .spawn(SpriteSheetComponents {
                scale: Scale(4.0),
                translation: Translation(Vec3::new(150.0, 0.0, 0.0)),
                sprite: TextureAtlasSprite::new(vendor_index as u32),
                texture_atlas: atlas_handle,
                ..Default::default()
            })
            // draw the atlas itself
            .spawn(SpriteComponents {
                material: materials.add(texture_atlas_texture.into()),
                translation: Vec3::new(-300.0, 0., 0.0).into(),
                ..Default::default()
            });

        rpg_sprite_handles.atlas_loaded = true;
    }
}
