use std::sync::Mutex;

use crate::tonemapping::ViewTonemappingPipeline;

use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    frame_graph::{
        render_pass_builder::RenderPassBuilder, ColorAttachmentDrawing, FrameGraph,
        FrameGraphTexture, GraphResourceNodeHandle, TextureViewDrawing, TextureViewInfo,
    },
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{LoadOp, Operations, PipelineCache, StoreOp},
    texture::{FallbackImage, GpuImage},
    view::{ViewTarget, ViewUniformOffset, ViewUniforms},
};

use super::{get_lut_bindings, Tonemapping, TonemappingLuts, TonemappingPipeline};

#[derive(Default)]
pub struct TonemappingNode {
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
        let view_uniforms_resource = world.resource::<ViewUniforms>();
        let view_uniforms = &view_uniforms_resource.uniforms;
        let _view_uniforms_id = view_uniforms.buffer().unwrap().id();

        if *tonemapping == Tonemapping::None {
            return Ok(());
        }

        if !target.is_hdr() {
            return Ok(());
        }

        let Some(_) = pipeline_cache.get_render_pipeline(view_tonemapping_pipeline.0) else {
            return Ok(());
        };

        let post_process = target.post_process_write();

        let source: GraphResourceNodeHandle<FrameGraphTexture> =
            frame_graph.get(post_process.source)?;
        let destination: GraphResourceNodeHandle<FrameGraphTexture> =
            frame_graph.get(post_process.destination)?;

        let mut last_tonemapping = self.last_tonemapping.lock().unwrap();

        let tonemapping_changed = if let Some(last_tonemapping) = &*last_tonemapping {
            tonemapping != last_tonemapping
        } else {
            true
        };
        if tonemapping_changed {
            *last_tonemapping = Some(*tonemapping);
        }

        let tonemapping_luts = world.resource::<TonemappingLuts>();

        let lut_bindings =
            get_lut_bindings(gpu_images, tonemapping_luts, tonemapping, fallback_image);

        let mut pass_node_builder = frame_graph.create_pass_node_bulder("main_opaque_pass_2d");

        let bing_group = pass_node_builder
            .create_bind_group_drawing_builder(
                None,
                tonemapping_pipeline.texture_bind_group.clone(),
            )
            .push_bind_group_entry(view_uniforms)
            .push_bind_group_entry(&source)
            .push_bind_group_entry(&tonemapping_pipeline.sampler_info)
            .push_bind_group_entry(lut_bindings.0)
            .push_bind_group_entry(lut_bindings.1)
            .build();

        let destination_read = pass_node_builder.read(destination);

        let mut builder = RenderPassBuilder::new(pass_node_builder);

        builder
            .add_color_attachment(ColorAttachmentDrawing {
                view: TextureViewDrawing {
                    texture: destination_read,
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
