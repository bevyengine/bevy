use bevy::{
    prelude::*,
    render::{
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext},
        render_resource::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor},
        renderer::{RenderContext, RenderDevice, RenderGraphRunner, RenderQueue},
        view::{ExtractedWindows, ViewTarget},
        RenderApp, RenderGraphPlugin, RenderStage, extract_resource::{ExtractResource, ExtractResourcePlugin},
    },
};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugin(bevy::asset::AssetPlugin::default())
        .add_plugin(bevy::input::InputPlugin)
        .add_plugin(bevy::window::WindowPlugin::default())
        .add_plugin(bevy::winit::WinitPlugin)
        .add_plugin(RenderGraphTest)
        .run();
}

struct RenderGraphTest;

impl Plugin for RenderGraphTest {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenderGraphPlugin);
        // the WindowRenderPlugin acquires swapchain images,
        // these need to be dropped again! (done by the TestNode)
        app.add_plugin(bevy::render::view::WindowRenderPlugin);
        
        app.insert_resource(MyTimer(0.0))
            .add_system(timer_advance)
            .add_plugin(ExtractResourcePlugin::<MyTimer>::default());

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app.add_system_to_stage(
            RenderStage::Render,
            render_system.at_end(),
        );

        let graph = &mut render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node(TestNode::NAME, TestNode);
    }
}

#[derive(Resource, ExtractResource, Clone)]
struct MyTimer(f64);

impl MyTimer {
    const TICK: f64 = 1.0 / 60.0;
}

fn timer_advance(mut timer: ResMut<MyTimer>) {
    timer.0 += MyTimer::TICK;
}

struct TestNode;

impl TestNode {
    const NAME: &'static str = "test_node";
}

impl Node for TestNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let timer = world.resource::<MyTimer>().0;
        
        let color = Color::rgb(
            (timer.sin() * 0.5 + 0.5) as f32,
            ((timer * 0.78).sin() * 0.5 + 0.5) as f32,
            ((timer * 0.63).sin() * 0.5 + 0.5) as f32);

        for (_id, window) in world.resource::<ExtractedWindows>().iter() {
            // NOTE: this is important, otherwise swap chain images are not dropped
            let swap_chain_texture = if let Some(swap_chain_texture) = &window.swap_chain_texture {
                swap_chain_texture
            } else {
                continue;
            };

            let pass_descriptor = RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: swap_chain_texture,
                    resolve_target: None,
                    ops: Operations {
                        // NOTE: re-export wgpu::Color?
                        load: LoadOp::Clear(color.into()),
                        store: true,
                    },
                })],
                ..Default::default()
            };

            render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}

// NOTE: this is just the default `render_system` minus error handling and the timer channel
fn render_system(world: &mut World) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });
    let graph = world.resource::<RenderGraph>();
    let render_device = world.resource::<RenderDevice>();
    let render_queue = world.resource::<RenderQueue>();

    RenderGraphRunner::run(graph, render_device.clone(), &render_queue.0, world).unwrap();

    let view_entities = world
        .query_filtered::<Entity, With<ViewTarget>>()
        .iter(world)
        .collect::<Vec<_>>();
    for view_entity in view_entities {
        world.entity_mut(view_entity).remove::<ViewTarget>();
    }

    let mut windows = world.resource_mut::<ExtractedWindows>();
    for window in windows.values_mut() {
        if let Some(texture_view) = window.swap_chain_texture.take() {
            if let Some(surface_texture) = texture_view.take_surface_texture() {
                surface_texture.present();
            }
        }
    }
}
