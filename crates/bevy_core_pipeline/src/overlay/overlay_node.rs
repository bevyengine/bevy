use std::mem;

use bevy_ecs::prelude::{FromWorld, QueryState, With, World, Resource};
use bevy_math::Vec4;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::TrackedRenderPass,
    render_resource::{
        encase, BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferInitDescriptor,
        BufferUsages, CachedRenderPipelineId, LoadOp, Operations, PipelineCache,
        RenderPassDescriptor,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::ViewTarget,
};

use super::{pipeline::OverlayPipeline, CameraOverlay, OverlayDiagnostics};

#[derive(Resource)]
pub(crate) struct DiagnosticOverlayBuffer {
    buffer: Buffer,
    bind_group: BindGroup,
}

pub(crate) mod graph {
    pub const NAME: &str = "OVERLAY";
    pub const NODE: &str = "OVERLAY_PASS";
    pub const NODE_INPUT: &str = "OVERLAY_PASS_VIEW";
    pub const IN_VIEW: &str = "OVERLAY_IN_VIEW";
}

impl FromWorld for DiagnosticOverlayBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let overlay_pipeline = world.get_resource::<OverlayPipeline>().unwrap();

        let byte_buffer = [0u8; mem::size_of::<Vec4>()];
        let mut buffer = encase::UniformBuffer::new(byte_buffer);
        buffer.write(&[Vec4::ZERO]).unwrap();
        let diagnostics_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("diagnostics Buffer"),
            contents: buffer.as_ref(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let diagnostics_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            layout: &overlay_pipeline.layout[0],
            entries: &[BindGroupEntry {
                binding: 0,
                resource: diagnostics_buffer.as_entire_binding(),
            }],
            label: Some("diagnostics_bind_group"),
        });

        DiagnosticOverlayBuffer {
            buffer: diagnostics_buffer,
            bind_group: diagnostics_bind_group,
        }
    }
}

impl DiagnosticOverlayBuffer {
    pub(crate) fn write_buffer(
        &self,
        diagnostics: &OverlayDiagnostics,
        render_queue: &RenderQueue,
    ) {
        let byte_buffer = [0u8; mem::size_of::<Vec4>()];
        let mut buffer = encase::UniformBuffer::new(byte_buffer);
        buffer
            .write(&[Vec4::new(diagnostics.avg_fps, 0.0, 0.0, 0.0)])
            .unwrap();
        render_queue.write_buffer(&self.buffer, 0, buffer.as_ref());
    }
}

pub(crate) struct OverlayNode {
    query: QueryState<&'static ViewTarget, With<CameraOverlay>>,
    render_pipeline: CachedRenderPipelineId,
}
impl OverlayNode {
    pub(crate) fn new(world: &mut World) -> Self {
        let overlay_pipeline = (*world.get_resource::<OverlayPipeline>().unwrap()).clone();
        let render_pipeline = world
            .resource_mut::<PipelineCache>()
            .queue_render_pipeline(overlay_pipeline.get_pipeline());

        Self {
            query: world.query_filtered(),
            render_pipeline,
        }
    }
}

impl Node for OverlayNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(graph::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(graph::IN_VIEW)?;

        let target = if let Ok(result) = self.query.get_manual(world, view_entity) {
            result
        } else {
            return Ok(());
        };

        // TODO: What does this do?
        // let target = ViewTarget {
        //     view: target.view.clone(),
        //     sampled_target: None,
        // };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("overlay"),
            color_attachments: &[Some(target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            }))],
            depth_stencil_attachment: None,
        };

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        let mut tracked = TrackedRenderPass::new(render_pass);

        let render_pipeline = world
            .resource::<PipelineCache>()
            .get_render_pipeline(self.render_pipeline)
            .unwrap();

        let buffer = world.resource::<DiagnosticOverlayBuffer>();

        tracked.set_render_pipeline(render_pipeline);
        tracked.set_bind_group(0, &buffer.bind_group, &[]);

        tracked.draw(0..3, 0..1);

        Ok(())
    }
}
