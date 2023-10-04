use std::sync::Mutex;

use crate::contrast_adaptive_sharpening::ViewCASPipeline;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferId, Operations,
        PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

use super::{CASPipeline, CASUniform};

pub struct CASNode {
    query: QueryState<
        (
            &'static ViewTarget,
            &'static ViewCASPipeline,
            &'static DynamicUniformIndex<CASUniform>,
        ),
        With<ExtractedView>,
    >,
    cached_bind_group: Mutex<Option<(BufferId, TextureViewId, BindGroup)>>,
}

impl FromWorld for CASNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_bind_group: Mutex::new(None),
        }
    }
}

impl Node for CASNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
        let pipeline_cache = world.resource::<PipelineCache>();
        let sharpening_pipeline = world.resource::<CASPipeline>();
        let uniforms = world.resource::<ComponentUniforms<CASUniform>>();

        let Ok((target, pipeline, uniform_index)) = self.query.get_manual(world, view_entity)
        else {
            return Ok(());
        };

        let uniforms_id = uniforms.buffer().unwrap().id();
        let Some(uniforms) = uniforms.binding() else {
            return Ok(());
        };

        let pipeline = pipeline_cache.get_render_pipeline(pipeline.0).unwrap();

        let view_target = target.post_process_write();
        let source = view_target.source;
        let destination = view_target.destination;

        let mut cached_bind_group = self.cached_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((buffer_id, texture_id, bind_group))
                if source.id() == *texture_id && uniforms_id == *buffer_id =>
            {
                bind_group
            }
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
                                    resource: uniforms,
                                },
                            ],
                        });

                let (_, _, bind_group) =
                    cached_bind_group.insert((uniforms_id, source.id(), bind_group));
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
        render_pass.set_bind_group(0, bind_group, &[uniform_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
