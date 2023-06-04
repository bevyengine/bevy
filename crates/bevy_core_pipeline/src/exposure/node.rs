use std::sync::Mutex;

use crate::exposure::{AutoExposurePipeline, AutoExposureStorage, ViewAutoExposurePipeline};
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryState;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferBinding,
        ComputePassDescriptor, PipelineCache, ShaderType, TextureViewId,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};

pub struct AutoExposureNode {
    query: QueryState<
        (
            &'static ViewUniformOffset,
            &'static ViewTarget,
            &'static ViewAutoExposurePipeline,
            &'static ExtractedView,
        ),
        With<ExtractedView>,
    >,
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl FromWorld for AutoExposureNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
            cached_texture_bind_group: Mutex::new(None),
        }
    }
}

impl Node for AutoExposureNode {
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
        let auto_exposure_pipeline = world.resource::<AutoExposurePipeline>();
        let storage_buffer = world.resource::<AutoExposureStorage>();

        if storage_buffer.0.is_none() {
            return Ok(());
        }

        let view_uniforms_resource = world.resource::<ViewUniforms>();
        let view_uniforms = &view_uniforms_resource.uniforms;
        let view_uniforms_buffer = view_uniforms.buffer().unwrap();

        let (view_uniform_offset, target, pipeline, view) =
            match self.query.get_manual(world, view_entity) {
                Ok(result) => result,
                Err(_) => return Ok(()),
            };

        let pipeline0 = pipeline_cache.get_compute_pipeline(pipeline.0[0]).unwrap();
        let pipeline1 = pipeline_cache.get_compute_pipeline(pipeline.0[1]).unwrap();

        let main_texture = target.main_texture_view();
        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if main_texture.id() == *id => bind_group,
            cached_bind_group => {
                let bind_group =
                    render_context
                        .render_device()
                        .create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: &auto_exposure_pipeline.bind_group,
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: BindingResource::Buffer(BufferBinding {
                                        buffer: view_uniforms_buffer,
                                        size: Some(ViewUniform::min_size()),
                                        offset: 0,
                                    }),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::TextureView(main_texture),
                                },
                                BindGroupEntry {
                                    binding: 2,
                                    resource: BindingResource::Buffer(BufferBinding {
                                        buffer: storage_buffer.0.as_ref().unwrap(),
                                        size: None,
                                        offset: 0,
                                    }),
                                },
                            ],
                        });

                let (_, bind_group) = cached_bind_group.insert((main_texture.id(), bind_group));
                bind_group
            }
        };

        let pass_descriptor = ComputePassDescriptor {
            label: Some("autoexposure_pass"),
        };

        let encoder = render_context.command_encoder();
        encoder.clear_buffer(storage_buffer.0.as_ref().unwrap(), 0, None);

        let mut compute_pass = encoder.begin_compute_pass(&pass_descriptor);
        compute_pass.set_pipeline(pipeline0);
        compute_pass.set_bind_group(0, bind_group, &[view_uniform_offset.offset]);
        compute_pass.dispatch_workgroups(
            (view.viewport.z + 15) / 16,
            (view.viewport.w + 15) / 16,
            1,
        );
        compute_pass.set_pipeline(pipeline1);
        compute_pass.set_bind_group(0, bind_group, &[view_uniform_offset.offset]);
        compute_pass.dispatch_workgroups(1, 1, 1);

        Ok(())
    }
}
