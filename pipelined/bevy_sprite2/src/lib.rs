mod bundle;
mod render;
mod rect;
mod sprite;

pub use rect::*;
pub use render::*;
pub use sprite::*;
pub use bundle::*;

use bevy_app::prelude::*;
use bevy_ecs::prelude::IntoSystem;
use bevy_render2::{
    main_pass::{DrawFunctions, MainPassPlugin},
    render_graph::RenderGraph,
    RenderStage,
};

#[derive(Default)]
pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Sprite>();
        app.add_plugin(MainPassPlugin);
        let render_app = app.sub_app_mut(0);
        render_app
            .add_system_to_stage(RenderStage::Extract, render::extract_sprites.system())
            .add_system_to_stage(RenderStage::Prepare, render::prepare_sprites.system())
            .add_system_to_stage(RenderStage::Queue, queue_sprites.system())
            .init_resource::<SpriteShaders>()
            .init_resource::<SpriteBuffers>();
        let draw_sprite = DrawSprite::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions>()
            .unwrap()
            .add(draw_sprite);
        let render_world = app.sub_app_mut(0).world.cell();
        let mut graph = render_world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("sprite", SpriteNode);
        graph.add_node_edge("sprite", "main_pass").unwrap();
        graph
            .add_node_edge("render_command_queue", "main_pass")
            .unwrap();
    }
}
