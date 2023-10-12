#![allow(clippy::type_complexity)]

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
        texture_atlas::{TextureAtlas, TextureAtlasLayout},
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

#[derive(Default)]
pub struct SpritePlugin;

pub const SPRITE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(2763343953151597127);

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
        app.init_asset::<TextureAtlasLayout>()
            .register_asset_reflect::<TextureAtlasLayout>()
            .register_type::<Sprite>()
            .register_type::<Anchor>()
            .register_type::<TextureAtlas>()
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
/// and without a [`NoFrustumCulling`] component.
///
/// Used in system set [`VisibilitySystems::CalculateBounds`].
pub fn calculate_bounds_2d(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    meshes_without_aabb: Query<(Entity, &Mesh2dHandle), (Without<Aabb>, Without<NoFrustumCulling>)>,
    sprites_without_aabb: Query<
        (Entity, &Sprite, &Handle<Image>, Option<&TextureAtlas>),
        (Without<Aabb>, Without<NoFrustumCulling>),
    >,
) {
    for (entity, mesh_handle) in &meshes_without_aabb {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some(aabb) = mesh.compute_aabb() {
                commands.entity(entity).insert(aabb);
            }
        }
    }
    for (entity, sprite, texture_handle, atlas) in &sprites_without_aabb {
        if let Some(size) = sprite.custom_size.or_else(|| match atlas {
            // We default to the texture size for regular sprites
            None => images.get(texture_handle).map(|image| image.size()),
            // We default to the drawn rect for atlas sprites
            Some(atlas) => atlas
                .texture_rect(&atlases)
                .map(|rect| (rect.min - rect.max).abs()),
        }) {
            let aabb = Aabb {
                center: (-sprite.anchor.as_vec() * size).extend(0.0).into(),
                half_extents: (0.5 * size).extend(0.0).into(),
            };
            commands.entity(entity).insert(aabb);
        }
    }
}
