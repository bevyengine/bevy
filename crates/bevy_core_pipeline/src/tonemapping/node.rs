use std::sync::Mutex;

use crate::tonemapping::ViewTonemappingPipeline;

use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    frame_graph::{
        ColorAttachmentDrawing, FrameGraph, PassBuilder, TextureViewDrawing, TextureViewInfo,
    },
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{LoadOp, Operations, PipelineCache, StoreOp},
    texture::{FallbackImage, GpuImage},
    view::{ViewTarget, ViewUniformOffset, ViewUniforms},
};

use super::{get_lut_image, Tonemapping, TonemappingLuts, TonemappingPipeline};

#[derive(Default)]
pub struct TonemappingNode;

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
        frame_graph: &mut FrameGraph,
        (view_uniform_offset, target, view_tonemapping_pipeline, tonemapping): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let tonemapping_pipeline = world.resource::<TonemappingPipeline>();
        let gpu_images = world.get_resource::<RenderAssets<GpuImage>>().unwrap();
        let fallback_image = world.resource::<FallbackImage>();
        let view_uniforms = world.resource::<ViewUniforms>();

        if *tonemapping == Tonemapping::None {
            return Ok(());
        }

        if !target.is_hdr() {
            return Ok(());
        }

        let (Some(_), Some(view_uniforms_binding)) = (
            pipeline_cache.get_render_pipeline(view_tonemapping_pipeline.0),
            view_uniforms
                .uniforms
                .make_binding_resource_handle(frame_graph),
        ) else {
            return Ok(());
        };

        let post_process = target.post_process_write();

        let source = post_process.source;
        let destination = post_process.destination;

        let tonemapping_luts = world.resource::<TonemappingLuts>();

        let lut_image = get_lut_image(gpu_images, tonemapping_luts, tonemapping, fallback_image);

        let mut pass_builder =
            PassBuilder::new(frame_graph.create_pass_node_bulder("tonemapping_pass"));

        let bing_group = pass_builder
            .create_bind_group_builder(None, tonemapping_pipeline.texture_bind_group.clone())
            .push_bind_group_entry(&view_uniforms_binding)
            .push_bind_group_entry(source)
            .push_bind_group_handle(&tonemapping_pipeline.sampler)
            .push_bind_group_entry(&lut_image.texture)
            .push_bind_group_handle(&lut_image.sampler)
            .build();

        let destination = pass_builder.write_material(destination);

        let mut builder = pass_builder.create_render_pass_builder();

        builder
            .set_pass_name("tonemapping_pass")
            .add_color_attachment(ColorAttachmentDrawing {
                view: TextureViewDrawing {
                    texture: destination,
                    desc: TextureViewInfo::default(),
                },
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Default::default()), // TODO shouldn't need to be cleared
                    store: StoreOp::Store,
                },
            })
            .set_render_pipeline(view_tonemapping_pipeline.0)
            .set_bind_group(0, bing_group, &[view_uniform_offset.offset])
            .draw(0..3, 0..1);

        Ok(())
    }
}
