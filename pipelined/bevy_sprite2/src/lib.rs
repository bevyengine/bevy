mod bundle;
mod dynamic_texture_atlas_builder;
mod rect;
mod render;
mod sprite;
mod texture_atlas;
mod texture_atlas_builder;

pub use bundle::*;
pub use dynamic_texture_atlas_builder::*;
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
use bevy_render2::{
    render_phase::DrawFunctions,
    render_resource::{Shader, SpecializedPipelines},
    RenderApp, RenderStage,
};

#[derive(Default)]
pub struct SpritePlugin;

pub const SPRITE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2763343953151597127);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum SpriteSystem {
    ExtractSprite,
}

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let sprite_shader = Shader::from_wgsl(include_str!("render/sprite.wgsl"));
        shaders.set_untracked(SPRITE_SHADER_HANDLE, sprite_shader);
        app.add_asset::<TextureAtlas>().register_type::<Sprite>();
        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<ImageBindGroups>()
            .init_resource::<SpritePipeline>()
            .init_resource::<SpecializedPipelines<SpritePipeline>>()
            .init_resource::<SpriteMeta>()
            .init_resource::<ExtractedSprites>()
            .init_resource::<SpriteAssetEvents>()
            .add_system_to_stage(
                RenderStage::Extract,
                render::extract_sprites.label(SpriteSystem::ExtractSprite),
            )
            .add_system_to_stage(RenderStage::Extract, render::extract_sprite_events)
            .add_system_to_stage(RenderStage::Prepare, render::prepare_sprites)
            .add_system_to_stage(RenderStage::Queue, queue_sprites);

        let draw_sprite = DrawSprite::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions<Transparent2d>>()
            .unwrap()
            .write()
            .add(draw_sprite);
    }
}
