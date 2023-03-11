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
use bevy_asset::{AddAsset, Assets, HandleUntyped};
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_render::{
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    ExtractSchedule, RenderApp, RenderSet,
};

#[derive(Default)]
pub struct SpritePlugin;

pub const SPRITE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2763343953151597127);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystem {
    ExtractSprites,
}

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world.resource_mut::<Assets<Shader>>();
        let sprite_shader = Shader::from_wgsl(include_str!("render/sprite.wgsl"));
        shaders.set_untracked(SPRITE_SHADER_HANDLE, sprite_shader);
        app.add_asset::<TextureAtlas>()
            .register_asset_reflect::<TextureAtlas>()
            .register_type::<Sprite>()
            .register_type::<Anchor>()
            .register_type::<Mesh2dHandle>()
            .add_plugin(Mesh2dRenderPlugin)
            .add_plugin(ColorMaterialPlugin);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpritePipeline>()
                .init_resource::<SpecializedRenderPipelines<SpritePipeline>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .init_resource::<SpriteAssetEvents>()
                .add_render_command::<Transparent2d, DrawSprite>()
                .add_systems(
                    (
                        extract_sprites.in_set(SpriteSystem::ExtractSprites),
                        extract_sprite_events,
                    )
                        .in_schedule(ExtractSchedule),
                )
                .add_system(
                    queue_sprites
                        .in_set(RenderSet::Queue)
                        .ambiguous_with(queue_material2d_meshes::<ColorMaterial>),
                );
        };
    }
}
