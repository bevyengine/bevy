mod bundle;
mod dynamic_texture_atlas_builder;
mod mesh2d;
mod rect;
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
pub use rect::*;
pub use render::*;
pub use sprite::*;
pub use texture_atlas::*;
pub use texture_atlas_builder::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets, HandleUntyped};
use bevy_core_pipeline::Transparent2d;
use bevy_ecs::schedule::{ParallelSystemDescriptorCoercion, SystemLabel};
use bevy_reflect::TypeUuid;
use bevy_render::{
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedPipelines},
    RenderApp, RenderStage,
};

#[derive(Default)]
pub struct SpritePlugin;

pub const SPRITE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2763343953151597127);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum SpriteSystem {
    ExtractSprites,
}

#[cfg(feature = "bevy_shader_hot_reloading")]
pub struct SpriteShaders {
    sprite_shader_handle: Handle<Shader>,
}

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        #[cfg(not(feature = "bevy_shader_hot_reloading"))]
        {
            shaders.set_untracked(
                SPRITE_SHADER_HANDLE,
                Shader::from_wgsl(include_str!(
                    "../../../assets/shaders/bevy_sprite/sprite.wgsl"
                )),
            );
        }
        #[cfg(feature = "bevy_shader_hot_reloading")]
        {
            let asset_server = app.world.get_resource::<AssetServer>().unwrap();
            let sprite_shader_handle: Handle<Shader> =
                asset_server.load("shaders/bevy_sprite/sprite.wgsl");
            shaders.add_alias(&sprite_shader_handle, SPRITE_SHADER_HANDLE);

            // NOTE: We need to store the strong handles created from the asset paths
            app.world.insert_resource(SpriteShaders {
                sprite_shader_handle,
            });
        }

        app.add_asset::<TextureAtlas>()
            .register_type::<Sprite>()
            .add_plugin(Mesh2dRenderPlugin)
            .add_plugin(ColorMaterialPlugin);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpritePipeline>()
                .init_resource::<SpecializedPipelines<SpritePipeline>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .init_resource::<SpriteAssetEvents>()
                .add_render_command::<Transparent2d, DrawSprite>()
                .add_system_to_stage(
                    RenderStage::Extract,
                    render::extract_sprites.label(SpriteSystem::ExtractSprites),
                )
                .add_system_to_stage(RenderStage::Extract, render::extract_sprite_events)
                .add_system_to_stage(RenderStage::Queue, queue_sprites);
        };
    }
}
