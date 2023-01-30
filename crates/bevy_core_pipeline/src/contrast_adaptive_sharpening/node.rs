use std::sync::Mutex;

use crate::contrast_adaptive_sharpening::ViewContrastAdaptiveSharpeningPipeline;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    extract_component::ComponentUniforms,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

use super::{CASUniform, ContrastAdaptiveSharpeningPipeline};

pub struct ContrastAdaptiveSharpeningNode {
    query: QueryState<
        (
            &'static ViewTarget,
            &'static ViewContrastAdaptiveSharpeningPipeline,
            // &'static CASUniform,
        ),
        With<ExtractedView>,
    >,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl ContrastAdaptiveSharpeningNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_texture_bind_group: Mutex::new(None),
        }
    }
}

impl Node for ContrastAdaptiveSharpeningNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(
            ContrastAdaptiveSharpeningNode::IN_VIEW,
            SlotType::Entity,
        )]
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
        let sharpening_pipeline = world.resource::<ContrastAdaptiveSharpeningPipeline>();
        let uniforms = world.resource::<ComponentUniforms<CASUniform>>();

        let (target, pipeline) = match self.query.get_manual(world, view_entity) {
            Ok(result) => result,
            Err(_) => return Ok(()),
        };

        let pipeline = pipeline_cache.get_render_pipeline(pipeline.0).unwrap();

        let view_target = target.post_process_write();

        let source = view_target.source;
        let destination = view_target.destination;
        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if source.id() == *id => bind_group,
            cached_bind_group => {
                let bind_group =
                    render_context
                        .render_device()
                        .create_bind_group(&BindGroupDescriptor {
                            label: Some("cas_bind_group"),
                            layout: &sharpening_pipeline.texture_bind_group,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::TextureView(view_target.source),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::Sampler(
                                        &sharpening_pipeline.sampler,
                                    ),
                                },
                                BindGroupEntry {
                                    binding: 2,
                                    resource: uniforms.binding().unwrap(),
                                },
                            ],
                        });

                let (_, bind_group) = cached_bind_group.insert((source.id(), bind_group));
                bind_group
            }
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("contrast_adaptive_sharpening"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
