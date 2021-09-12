mod bundle;
mod dynamic_texture_atlas_builder;
mod rect;
mod render;
mod sprite;
mod texture_atlas;
mod texture_atlas_builder;

use bevy_asset::AddAsset;
pub use bundle::*;
pub use dynamic_texture_atlas_builder::*;
pub use rect::*;
pub use render::*;
pub use sprite::*;
pub use texture_atlas::*;
pub use texture_atlas_builder::*;

use bevy_app::prelude::*;
use bevy_render2::{
    render_graph::RenderGraph, render_phase::DrawFunctions, RenderApp, RenderStage,
};

#[derive(Default)]
pub struct Sprite2dPlugin;

impl Plugin for Sprite2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TextureAtlas>().register_type::<Sprite2d>();
        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<ExtractedSprites2d>()
            .add_system_to_stage(RenderStage::Extract, render::extract_atlases_2d)
            .add_system_to_stage(RenderStage::Extract, render::extract_sprites_2d)
            .add_system_to_stage(RenderStage::Prepare, render::prepare_sprites_2d)
            .add_system_to_stage(RenderStage::Queue, render::queue_sprites_2d)
            .init_resource::<Sprite2dShaders>()
            .init_resource::<Sprite2dMeta>();
        let draw_sprite_2d = DrawSprite2d::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions>()
            .unwrap()
            .write()
            .add(draw_sprite_2d);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("sprite2d", Sprite2dNode);
        graph
            .add_node_edge("sprite2d", bevy_core_pipeline::node::MAIN_PASS_DEPENDENCIES)
            .unwrap();
    }
}

#[derive(Default)]
pub struct Sprite3dPlugin;

impl Plugin for Sprite3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TextureAtlas>().register_type::<Sprite3d>();
        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<ExtractedSprites3d>()
            .add_system_to_stage(RenderStage::Extract, render::extract_atlases_3d)
            .add_system_to_stage(RenderStage::Extract, render::extract_sprites_3d)
            .add_system_to_stage(RenderStage::Prepare, render::prepare_sprites_3d)
            .add_system_to_stage(RenderStage::Queue, render::queue_sprites_3d)
            .init_resource::<Sprite3dShaders>()
            .init_resource::<Sprite3dMeta>();
        let draw_sprite_3d = DrawSprite3d::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions>()
            .unwrap()
            .write()
            .add(draw_sprite_3d);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("sprite3d", Sprite3dNode);
        graph
            .add_node_edge("sprite3d", bevy_core_pipeline::node::MAIN_PASS_DEPENDENCIES)
            .unwrap();
    }
}
