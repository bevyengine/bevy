use crate::contrast_adaptive_sharpening::ViewCasPipeline;
use bevy_ecs::prelude::*;
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    frame_graph::{ColorAttachment, FrameGraph, TextureView, TextureViewInfo},
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{Operations, PipelineCache},
    view::{ExtractedView, ViewTarget},
};

use super::{CasPipeline, CasUniform};

pub struct CasNode {
    query: QueryState<
        (
            &'static ViewTarget,
            &'static ViewCasPipeline,
            &'static DynamicUniformIndex<CasUniform>,
        ),
        With<ExtractedView>,
    >,
}

impl FromWorld for CasNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for CasNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
        let pipeline_cache = world.resource::<PipelineCache>();
        let sharpening_pipeline = world.resource::<CasPipeline>();
        let uniforms = world.resource::<ComponentUniforms<CasUniform>>();

        let Ok((target, pipeline, uniform_index)) = self.query.get_manual(world, view_entity)
        else {
            return Ok(());
        };

        let Some(uniforms_handle) = uniforms.make_binding_resource_handle(frame_graph) else {
            return Ok(());
        };

        let Some(_) = pipeline_cache.get_render_pipeline(pipeline.0) else {
            return Ok(());
        };

        let post_process = target.post_process_write();

        let bind_group = frame_graph
            .create_bind_group_handle_builder(
                Some("cas_bind_group".into()),
                &sharpening_pipeline.texture_bind_group,
            )
            .add_helper(0, post_process.source)
            .add_handle(1, &sharpening_pipeline.sampler)
            .add_handle(2, &uniforms_handle)
            .build();

        let mut pass_builder = frame_graph.create_pass_builder("cas_node");

        let destination = pass_builder.write_material(post_process.destination);

        pass_builder
            .create_render_pass_builder("contrast_adaptive_sharpening")
            .add_color_attachment(ColorAttachment {
                view: TextureView {
                    texture: destination,
                    desc: TextureViewInfo::default(),
                },
                resolve_target: None,
                ops: Operations::default(),
            })
            .set_render_pipeline(pipeline.0)
            .set_bind_group_handle(0, &bind_group, &[uniform_index.index()])
            .draw(0..3, 0..1);

        Ok(())
    }
}
