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
use bevy_asset::AddAsset;
use bevy_core_pipeline::Transparent2d;
use bevy_render2::{
    render_graph::RenderGraph, render_phase::DrawFunctions, RenderApp, RenderStage,
};

#[derive(Default)]
pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TextureAtlas>().register_type::<Sprite>();
        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<ImageBindGroups>()
            .init_resource::<SpriteShaders>()
            .init_resource::<SpriteMeta>()
            .add_system_to_stage(RenderStage::Extract, render::extract_atlases)
            .add_system_to_stage(RenderStage::Extract, render::extract_sprites)
            .add_system_to_stage(RenderStage::Prepare, render::prepare_sprites)
            .add_system_to_stage(RenderStage::Queue, queue_sprites);

        let draw_sprite = DrawSprite::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions<Transparent2d>>()
            .unwrap()
            .write()
            .add(draw_sprite);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("sprite", SpriteNode);
        graph
            .add_node_edge("sprite", bevy_core_pipeline::node::MAIN_PASS_DEPENDENCIES)
            .unwrap();
    }
}
