mod draw_state;
mod draw;

pub use draw_state::*;
pub use draw::*;

use crate::{RenderStage, pass::RenderPassColorAttachment, renderer::RenderContext};
use crate::{
    camera::CameraPlugin,
    color::Color,
    pass::{
        LoadOp, Operations, PassDescriptor, RenderPass,
        TextureAttachment,
    },
    render_graph::{Node, RenderGraph, ResourceSlotInfo, ResourceSlots, WindowSwapChainNode},
    render_resource::RenderResourceType,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_window::WindowId;
use std::borrow::Cow;

#[derive(Default)]
pub struct MainPassPlugin;

impl Plugin for MainPassPlugin {
    fn build(&self, app: &mut App) {
        // TODO: this should probably just be a dependency
        app.add_plugin(CameraPlugin);
        app.sub_app_mut(0)
            .add_system_to_stage(
                RenderStage::Prepare,
                clear_transparent_phase.exclusive_system().at_start(),
            )
            .init_resource::<RenderPhase>()
            .init_resource::<DrawFunctions>();
        let render_world = app.sub_app_mut(0).world.cell();
        let mut graph = render_world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("main_pass", MainPassNode);
        graph.add_node_edge("camera", "main_pass").unwrap();
        graph.add_node(
            "primary_swap_chain",
            WindowSwapChainNode::new(WindowId::primary()),
        );
        graph
            .add_slot_edge(
                "primary_swap_chain",
                WindowSwapChainNode::OUT_TEXTURE,
                "main_pass",
                MainPassNode::IN_COLOR_ATTACHMENT,
            )
            .unwrap();
    }
}

// TODO: sort out the best place for this
fn clear_transparent_phase(mut transparent_phase: ResMut<RenderPhase>) {
    // TODO: TRANSPARENT PHASE SHOULD NOT BE CLEARED HERE!
    transparent_phase.drawn_things.clear();
}

pub struct Drawable {
    pub draw_function: usize,
    pub draw_key: usize,
    pub sort_key: usize,
}

#[derive(Default)]
pub struct RenderPhase {
    drawn_things: Vec<Drawable>,
}

impl RenderPhase {
    #[inline]
    pub fn add(&mut self, drawable: Drawable) {
        self.drawn_things.push(drawable);
    }

    pub fn sort(&mut self) {
        self.drawn_things.sort_by_key(|d| d.sort_key);
    }
}

pub struct MainPassNode;

impl MainPassNode {
    pub const IN_COLOR_ATTACHMENT: &'static str = "color_attachment";
}

impl Node for MainPassNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        static INPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(MainPassNode::IN_COLOR_ATTACHMENT),
            resource_type: RenderResourceType::Texture,
        }];
        INPUT
    }
    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        // TODO: consider adding shorthand like `get_texture(0)`
        let color_attachment_texture = input.get(0).unwrap().get_texture().unwrap();
        let pass_descriptor = PassDescriptor {
            color_attachments: vec![RenderPassColorAttachment {
                attachment: TextureAttachment::Id(color_attachment_texture),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::rgb(0.4, 0.4, 0.4)),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
            sample_count: 1,
        };

        let transparent_phase = world.get_resource::<RenderPhase>().unwrap();
        let draw_functions = world.get_resource::<DrawFunctions>().unwrap();

        render_context.begin_pass(&pass_descriptor, &mut |render_pass: &mut dyn RenderPass| {
            let mut draw_functions = draw_functions.draw_function.lock();
            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for drawable in transparent_phase.drawn_things.iter() {
                draw_functions[drawable.draw_function].draw(
                    world,
                    &mut tracked_pass,
                    drawable.draw_key,
                    drawable.sort_key,
                );
            }
        })
    }
}
