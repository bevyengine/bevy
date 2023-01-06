use std::sync::Mutex;

use crate::tonemapping::{TonemappingPipeline, ViewTonemappingPipeline};
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, LoadOp, Operations,
        PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, SamplerDescriptor,
        TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

pub struct TonemappingNode {
    query: QueryState<(&'static ViewTarget, &'static ViewTonemappingPipeline), With<ExtractedView>>,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl TonemappingNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_texture_bind_group: Mutex::new(None),
        }
    }
}

impl Node for TonemappingNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(TonemappingNode::IN_VIEW, SlotType::Entity)]
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
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let pipeline_cache = world.resource::<PipelineCache>();
        let tonemapping_pipeline = world.resource::<TonemappingPipeline>();

        let (target, tonemapping) = match self.query.get_manual(world, view_entity) {
            Ok(result) => result,
            Err(_) => return Ok(()),
        };

        if !target.is_hdr() {
            return Ok(());
        }

        let pipeline = match pipeline_cache.get_render_pipeline(tonemapping.0) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if source.id() == *id => bind_group,
            cached_bind_group => {
                let sampler = render_context
                    .render_device
                    .create_sampler(&SamplerDescriptor::default());

                let bind_group =
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: &tonemapping_pipeline.texture_bind_group,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(source),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(&sampler),
                                },
                            ],
                        });

                let (_, bind_group) = cached_bind_group.insert((source.id(), bind_group));
                bind_group
            }
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("tonemapping_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Default::default()), // TODO shouldn't need to be cleared
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        };

        let mut render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
