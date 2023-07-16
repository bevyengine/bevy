use std::sync::Mutex;

use crate::fxaa::{CameraFxaaPipeline, Fxaa, FxaaPipeline};
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_render::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, FilterMode, Operations,
        PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, SamplerDescriptor,
        TextureViewId,
    },
    renderer::RenderContext,
    view::ViewTarget,
};
use bevy_utils::default;

#[derive(Default)]
pub struct FxaaNode {
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl ViewNode for FxaaNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraFxaaPipeline,
        &'static Fxaa,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, pipeline, fxaa): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let fxaa_pipeline = world.resource::<FxaaPipeline>();

        if !fxaa.enabled {
            return Ok(());
        };

        let pipeline = pipeline_cache
            .get_render_pipeline(pipeline.pipeline_id)
            .unwrap();

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;
        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if source.id() == *id => bind_group,
            cached_bind_group => {
                let sampler = render_context
                    .render_device()
                    .create_sampler(&SamplerDescriptor {
                        mipmap_filter: FilterMode::Linear,
                        mag_filter: FilterMode::Linear,
                        min_filter: FilterMode::Linear,
                        ..default()
                    });

                let bind_group =
                    render_context
                        .render_device()
                        .create_bind_group(&BindGroupDescriptor {
                            label: None,
                            layout: &fxaa_pipeline.texture_bind_group,
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
            label: Some("fxaa_pass"),
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
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
