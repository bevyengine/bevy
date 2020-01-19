use crate::prelude::*;
use crate::render::render_graph_2::{RenderGraph, WgpuRenderer, Renderer};
use winit::window::Window;

pub fn build_wgpu_render_system(world: &mut World) -> Box<dyn Schedulable> {
    let window = world.resources.get::<Window>();
    let renderer = WgpuRenderer {
        
    };

    SystemBuilder::new("wgpu_render_system")
        .read_resource::<RenderGraph>()
        .with_query(<(Write<Node>,)>::query().filter(!component::<Parent>()))
        .write_component::<Node>()
        .read_component::<Children>()
        .build(move |_, world, render_graph, node_query| {
            renderer.process_render_graph(*render_graph);
        })
}