use std::sync::Mutex;

use crate::tonemapping::{TonemappingLuts, TonemappingPipeline, ViewTonemappingPipeline};

use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferId, LoadOp,
        Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        SamplerDescriptor, TextureViewId,
    },
    renderer::RenderContext,
    texture::Image,
    view::{ViewTarget, ViewUniformOffset, ViewUniforms},
};

use super::{get_lut_bindings, Tonemapping};

#[derive(Default)]
pub struct TonemappingNode {
    cached_bind_group: Mutex<Option<(BufferId, TextureViewId, BindGroup)>>,
    last_tonemapping: Mutex<Option<Tonemapping>>,
}

impl ViewNode for TonemappingNode {
    type ViewQuery = (
        &'static ViewUniformOffset,
        &'static ViewTarget,
        &'static ViewTonemappingPipeline,
        &'static Tonemapping,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_uniform_offset, target, view_tonemapping_pipeline, tonemapping): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let tonemapping_pipeline = world.resource::<TonemappingPipeline>();
        let gpu_images = world.get_resource::<RenderAssets<Image>>().unwrap();
        let view_uniforms_resource = world.resource::<ViewUniforms>();
        let view_uniforms = &view_uniforms_resource.uniforms;
        let view_uniforms_id = view_uniforms.buffer().unwrap().id();

        if !target.is_hdr() {
            return Ok(());
        }

        let pipeline = match pipeline_cache.get_render_pipeline(view_tonemapping_pipeline.0) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let mut last_tonemapping = self.last_tonemapping.lock().unwrap();

        let tonemapping_changed = if let Some(last_tonemapping) = &*last_tonemapping {
            tonemapping != last_tonemapping
        } else {
            true
        };
        if tonemapping_changed {
            *last_tonemapping = Some(*tonemapping);
        }

        let mut cached_bind_group = self.cached_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((buffer_id, texture_id, bind_group))
                if view_uniforms_id == *buffer_id
                    && source.id() == *texture_id
                    && !tonemapping_changed =>
            {
                bind_group
            }
            cached_bind_group => {
                let sampler = render_context
                    .render_device()
                    .create_sampler(&SamplerDescriptor::default());

                let tonemapping_luts = world.resource::<TonemappingLuts>();

                let mut entries = vec![
                    BindGroupEntry {
                        binding: 0,
                        resource: view_uniforms.binding().unwrap(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(source),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&sampler),
                    },
                ];

                entries.extend(get_lut_bindings(
                    gpu_images,
                    tonemapping_luts,
                    tonemapping,
                    [3, 4],
                ));

                let bind_group =
                    render_context
                        .render_device()
                        .create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: &tonemapping_pipeline.texture_bind_group,
                            entries: &entries,
                        });

                let (_, _, bind_group) =
                    cached_bind_group.insert((view_uniforms_id, source.id(), bind_group));
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
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
