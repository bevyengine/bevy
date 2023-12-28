//! Provides 2D sprite rendering functionality.
mod bundle;
mod dynamic_texture_atlas_builder;
mod mesh2d;
mod render;
mod sprite;
mod texture_atlas;
mod texture_atlas_builder;

pub mod collide_aabb;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        bundle::{SpriteBundle, SpriteSheetBundle},
        sprite::Sprite,
        texture_atlas::{TextureAtlas, TextureAtlasSprite},
        ColorMaterial, ColorMesh2dBundle, TextureAtlasBuilder,
    };
}

pub use bundle::*;
pub use dynamic_texture_atlas_builder::*;
pub use mesh2d::*;
pub use render::*;
pub use sprite::*;
pub use texture_atlas::*;
pub use texture_atlas_builder::*;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetApp, Assets, Handle};
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::prelude::*;
use bevy_render::{
    mesh::Mesh,
    primitives::Aabb,
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    texture::Image,
    view::{NoFrustumCulling, VisibilitySystems},
    ExtractSchedule, Render, RenderApp, RenderSet,
};

/// Adds support for 2D sprite rendering.
#[derive(Default)]
pub struct SpritePlugin;

pub const SPRITE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(2763343953151597127);

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystem {
    ExtractSprites,
}

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SPRITE_SHADER_HANDLE,
            "render/sprite.wgsl",
            Shader::from_wgsl
        );
        app.init_asset::<TextureAtlas>()
            .register_asset_reflect::<TextureAtlas>()
            .register_type::<Sprite>()
            .register_type::<TextureAtlasSprite>()
            .register_type::<Anchor>()
            .register_type::<Mesh2dHandle>()
            .add_plugins((Mesh2dRenderPlugin, ColorMaterialPlugin))
            .add_systems(
                PostUpdate,
                calculate_bounds_2d.in_set(VisibilitySystems::CalculateBounds),
            );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpecializedRenderPipelines<SpritePipeline>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .init_resource::<SpriteAssetEvents>()
                .add_render_command::<Transparent2d, DrawSprite>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_sprites.in_set(SpriteSystem::ExtractSprites),
                        extract_sprite_events,
                    ),
                )
                .add_systems(
                    Render,
                    (
                        queue_sprites
                            .in_set(RenderSet::Queue)
                            .ambiguous_with(queue_material2d_meshes::<ColorMaterial>),
                        prepare_sprites.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        };
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<SpritePipeline>();
        }
    }
}

/// System calculating and inserting an [`Aabb`] component to entities with either:
/// - a `Mesh2dHandle` component,
/// - a `Sprite` and `Handle<Image>` components,
/// - a `TextureAtlasSprite` and `Handle<TextureAtlas>` components,
/// and without a [`NoFrustumCulling`] component.
///
/// Used in system set [`VisibilitySystems::CalculateBounds`].
pub fn calculate_bounds_2d(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlas>>,
    meshes_without_aabb: Query<(Entity, &Mesh2dHandle), (Without<Aabb>, Without<NoFrustumCulling>)>,
    sprites_to_recalculate_aabb: Query<
        (Entity, &Sprite, &Handle<Image>),
        (
            Or<(Without<Aabb>, Changed<Sprite>)>,
            Without<NoFrustumCulling>,
        ),
    >,
    atlases_without_aabb: Query<
        (Entity, &TextureAtlasSprite, &Handle<TextureAtlas>),
        (Without<Aabb>, Without<NoFrustumCulling>),
    >,
) {
    for (entity, mesh_handle) in &meshes_without_aabb {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some(aabb) = mesh.compute_aabb() {
                commands.entity(entity).try_insert(aabb);
            }
        }
    }
    for (entity, sprite, texture_handle) in &sprites_to_recalculate_aabb {
        if let Some(size) = sprite
            .custom_size
            .or_else(|| images.get(texture_handle).map(|image| image.size_f32()))
        {
            let aabb = Aabb {
                center: (-sprite.anchor.as_vec() * size).extend(0.0).into(),
                half_extents: (0.5 * size).extend(0.0).into(),
            };
            commands.entity(entity).try_insert(aabb);
        }
    }
    for (entity, atlas_sprite, atlas_handle) in &atlases_without_aabb {
        if let Some(size) = atlas_sprite.custom_size.or_else(|| {
            atlases
                .get(atlas_handle)
                .and_then(|atlas| atlas.textures.get(atlas_sprite.index))
                .map(|rect| (rect.min - rect.max).abs())
        }) {
            let aabb = Aabb {
                center: (-atlas_sprite.anchor.as_vec() * size).extend(0.0).into(),
                half_extents: (0.5 * size).extend(0.0).into(),
            };
            commands.entity(entity).try_insert(aabb);
        }
    }
}

#[cfg(test)]
mod test {

    use bevy_math::Vec2;
    use bevy_utils::default;

    use super::*;

    #[test]
    fn calculate_bounds_2d_create_aabb_for_image_sprite_entity() {
        // Setup app
        let mut app = App::new();

        // Add resources and get handle to image
        let mut image_assets = Assets::<Image>::default();
        let image_handle = image_assets.add(Image::default());
        app.insert_resource(image_assets);
        let mesh_assets = Assets::<Mesh>::default();
        app.insert_resource(mesh_assets);
        let texture_atlas_assets = Assets::<TextureAtlas>::default();
        app.insert_resource(texture_atlas_assets);

        // Add system
        app.add_systems(Update, calculate_bounds_2d);

        // Add entites
        let entity = app.world.spawn((Sprite::default(), image_handle)).id();

        // Verify that the entity does not have an AABB
        assert!(!app
            .world
            .get_entity(entity)
            .expect("Could not find entity")
            .contains::<Aabb>());

        // Run system
        app.update();

        // Verify the AABB exists
        assert!(app
            .world
            .get_entity(entity)
            .expect("Could not find entity")
            .contains::<Aabb>());
    }

    #[test]
    fn calculate_bounds_2d_update_aabb_when_sprite_custom_size_changes_to_some() {
        // Setup app
        let mut app = App::new();

        // Add resources and get handle to image
        let mut image_assets = Assets::<Image>::default();
        let image_handle = image_assets.add(Image::default());
        app.insert_resource(image_assets);
        let mesh_assets = Assets::<Mesh>::default();
        app.insert_resource(mesh_assets);
        let texture_atlas_assets = Assets::<TextureAtlas>::default();
        app.insert_resource(texture_atlas_assets);

        // Add system
        app.add_systems(Update, calculate_bounds_2d);

        // Add entites
        let entity = app
            .world
            .spawn((
                Sprite {
                    custom_size: Some(Vec2::ZERO),
                    ..default()
                },
                image_handle,
            ))
            .id();

        // Create initial AABB
        app.update();

        // Get the initial AABB
        let first_aabb = *app
            .world
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Could not find initial AABB");

        // Change `custom_size` of sprite
        let mut binding = app
            .world
            .get_entity_mut(entity)
            .expect("Could not find entity");
        let mut sprite = binding
            .get_mut::<Sprite>()
            .expect("Could not find sprite component of entity");
        sprite.custom_size = Some(Vec2::ONE);

        // Re-run the `calculate_bounds_2d` system to get the new AABB
        app.update();

        // Get the re-calculated AABB
        let second_aabb = *app
            .world
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Could not find second AABB");

        // Check that the AABBs are not equal
        assert_ne!(first_aabb, second_aabb);
    }
}
