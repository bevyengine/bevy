use bevy::{asset::LoadState, ecs::{Stage, State, StateStage, SystemStage}, prelude::*, sprite::TextureAtlasBuilder, utils::HashMap};
use std::{hash::Hash, ops::Deref, sync::RwLock};

/// In this example we generate a new texture atlas (sprite sheet) from a folder containing individual sprites
fn main() {
    App::build()
        .init_resource::<RpgSpriteHandles>()
        .add_resource(AppState::LoadingAssets)
        .add_plugins(DefaultPlugins)
        .add_system(load_atlas)
        .add_stage_after(
            stage::UPDATE,
            "state",
            StateStage::default()
                .state(AppState::Setup, SystemStage::parallel().system(setup))
                .enter_state(AppState::Setup, SystemStage::parallel().system(setup))
                .state(
                    AppState::LoadingAssets,
                    SystemStage::parallel().system(load_atlas),
                ),
        )
        .run();
}


#[derive(Clone, Hash, Eq, PartialEq)]
pub enum AppState {
    Setup,
    LoadingAssets,
    Finish,
}

#[derive(Default)]
pub struct RpgSpriteHandles {
    handles: Vec<HandleUntyped>,
    atlas_loaded: bool,
}

fn setup(
    state: Res<State<AppState>>,
    mut rpg_sprite_handles: ResMut<RpgSpriteHandles>,
    asset_server: Res<AssetServer>,
) {
    rpg_sprite_handles.handles = asset_server.load_folder("textures/rpg").unwrap();
    // state.set(AppState::LoadingAssets);
}

fn load_atlas(
    commands: &mut Commands,
    // mut state: ResMut<State<AppState>>,
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
            texture_atlas_builder.add_texture(handle.clone_weak().typed::<Texture>(), texture);
        }

        let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
        let texture_atlas_texture = texture_atlas.texture.clone();
        let vendor_handle =
            asset_server.get_handle("textures/rpg/chars/vendor/generic-rpg-vendor.png");
        let vendor_index = texture_atlas.get_texture_index(&vendor_handle).unwrap();
        let atlas_handle = texture_atlases.add(texture_atlas);

        // set up a scene to display our texture atlas
        commands
            .spawn(Camera2dBundle::default())
            // draw a sprite from the atlas
            .spawn(SpriteSheetBundle {
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
            .spawn(SpriteBundle {
                material: materials.add(texture_atlas_texture.into()),
                transform: Transform::from_translation(Vec3::new(-300.0, 0.0, 0.0)),
                ..Default::default()
            });

        rpg_sprite_handles.atlas_loaded = true;
    }
}
