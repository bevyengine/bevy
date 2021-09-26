pub mod collide_aabb;
pub mod entity;

mod color_material;
mod dynamic_texture_atlas_builder;
mod frustum_culling;
mod rect;
mod render;
mod sprite;
mod texture_atlas;
mod texture_atlas_builder;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        entity::{SpriteBundle, SpriteSheetBundle},
        ColorMaterial, Sprite, SpriteResizeMode, TextureAtlas, TextureAtlasSprite,
    };
}

pub use color_material::*;
pub use dynamic_texture_atlas_builder::*;
pub use rect::*;
pub use render::*;
pub use sprite::*;
pub use texture_atlas::*;
pub use texture_atlas_builder::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets, Handle, HandleUntyped};
use bevy_ecs::component::{ComponentDescriptor, StorageType};
use bevy_math::Vec2;
use bevy_reflect::TypeUuid;
use bevy_render::{
    draw::OutsideFrustum,
    mesh::{shape, Mesh},
    pipeline::PipelineDescriptor,
    render_graph::RenderGraph,
    shader::{asset_shader_defs_system, Shader},
};
use sprite::sprite_system;

#[derive(Debug, Clone)]
pub struct SpriteSettings {
    /// Enable sprite frustum culling.
    ///
    /// # Warning
    /// This is currently experimental. It does not work correctly in all cases.
    pub frustum_culling_enabled: bool,
}

impl Default for SpriteSettings {
    fn default() -> Self {
        Self {
            frustum_culling_enabled: false,
        }
    }
}

#[derive(Default)]
pub struct SpritePlugin;

pub const QUAD_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 14240461981130137526);

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<ColorMaterial>()
            .add_asset::<TextureAtlas>()
            .register_type::<Sprite>()
            .register_type::<SpriteResizeMode>()
            .add_system_to_stage(CoreStage::PostUpdate, sprite_system)
            .add_system_to_stage(CoreStage::PostUpdate, material_texture_detection_system)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                asset_shader_defs_system::<ColorMaterial>,
            );

        let sprite_settings = app
            .world
            .get_resource_or_insert_with(SpriteSettings::default)
            .clone();
        if sprite_settings.frustum_culling_enabled {
            app.add_system_to_stage(
                CoreStage::PostUpdate,
                frustum_culling::sprite_frustum_culling_system,
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                frustum_culling::atlas_frustum_culling_system,
            );
        }
        app.world
            .register_component(ComponentDescriptor::new::<OutsideFrustum>(
                StorageType::SparseSet,
            ))
            .unwrap();

        let world_cell = app.world.cell();
        let mut render_graph = world_cell.get_resource_mut::<RenderGraph>().unwrap();
        let mut pipelines = world_cell
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();
        let mut shaders = world_cell.get_resource_mut::<Assets<Shader>>().unwrap();
        crate::render::add_sprite_graph(&mut render_graph, &mut pipelines, &mut shaders);

        let mut meshes = world_cell.get_resource_mut::<Assets<Mesh>>().unwrap();
        let mut color_materials = world_cell
            .get_resource_mut::<Assets<ColorMaterial>>()
            .unwrap();
        color_materials.set_untracked(Handle::<ColorMaterial>::default(), ColorMaterial::default());
        meshes.set_untracked(
            QUAD_HANDLE,
            // Use a flipped quad because the camera is facing "forward" but quads should face
            // backward
            Mesh::from(shape::Quad::new(Vec2::new(1.0, 1.0))),
        )
    }
}
