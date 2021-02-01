pub mod collide_aabb;
pub mod entity;

mod color_material;
mod dynamic_texture_atlas_builder;
mod rect;
mod render;
mod sprite;
mod texture_atlas;
mod texture_atlas_builder;

use bevy_ecs::IntoSystem;
pub use color_material::*;
pub use dynamic_texture_atlas_builder::*;
pub use rect::*;
pub use render::*;
pub use sprite::*;
pub use texture_atlas::*;
pub use texture_atlas_builder::*;

pub mod prelude {
    pub use crate::{
        entity::{SpriteBundle, SpriteSheetBundle},
        ColorMaterial, Sprite, SpriteResizeMode, TextureAtlas, TextureAtlasSprite,
    };
}

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets, Handle, HandleUntyped};
use bevy_math::Vec2;
use bevy_reflect::{RegisterTypeBuilder, TypeUuid};
use bevy_render::{
    mesh::{shape, Mesh},
    render_graph::RenderGraph,
    shader::asset_shader_defs_system,
};
use sprite::sprite_system;

#[derive(Default)]
pub struct SpritePlugin;

pub const QUAD_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 14240461981130137526);

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<ColorMaterial>()
            .add_asset::<TextureAtlas>()
            .register_type::<Sprite>()
            .add_system_to_stage(stage::POST_UPDATE, sprite_system.system())
            .add_system_to_stage(
                stage::POST_UPDATE,
                asset_shader_defs_system::<ColorMaterial>.system(),
            );

        let resources = app.resources_mut();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_sprite_graph(resources);

        let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();

        let mut color_materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
        color_materials.set_untracked(Handle::<ColorMaterial>::default(), ColorMaterial::default());
        meshes.set_untracked(
            QUAD_HANDLE,
            // Use a flipped quad because the camera is facing "forward" but quads should face backward
            Mesh::from(shape::Quad::new(Vec2::new(1.0, 1.0))),
        )
    }
}
