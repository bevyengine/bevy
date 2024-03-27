use super::compensation_curve::AutoExposureCompensationCurve;
use crate::auto_exposure::{
    pipeline::{AutoExposurePipeline, ViewAutoExposurePipeline},
    AutoExposureResources,
};
use bevy_ecs::{
    query::QueryState,
    system::lifetimeless::Read,
    world::{FromWorld, World},
};
use bevy_render::{
    render_asset::RenderAssets,
    render_graph::*,
    render_resource::*,
    renderer::RenderContext,
    texture::{FallbackImage, Image},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct AutoExposure;

pub struct AutoExposureNode {
    query: QueryState<(
        Read<ViewUniformOffset>,
        Read<ViewTarget>,
        Read<ViewAutoExposurePipeline>,
        Read<ExtractedView>,
    )>,
}

impl FromWorld for AutoExposureNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
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
        let pipeline = world.resource::<AutoExposurePipeline>();
        let resources = world.resource::<AutoExposureResources>();

        let view_uniforms_resource = world.resource::<ViewUniforms>();
        let view_uniforms = &view_uniforms_resource.uniforms;
        let view_uniforms_buffer = view_uniforms.buffer().unwrap();

        let Ok((view_uniform_offset, view_target, auto_exposure, view)) =
            self.query.get_manual(world, view_entity)
        else {
            return Ok(());
        };

        let histogram_pipeline = pipeline_cache
            .get_compute_pipeline(auto_exposure.histogram_pipeline)
            .unwrap();
        let average_pipeline = pipeline_cache
            .get_compute_pipeline(auto_exposure.mean_luminance_pipeline)
            .unwrap();

        let source = view_target.main_texture_view();

        let fallback = world.resource::<FallbackImage>();
        let mask = world
            .resource::<RenderAssets<Image>>()
            .get(&auto_exposure.metering_mask);
        let mask = mask
            .map(|i| &i.texture_view)
            .unwrap_or(&fallback.d2.texture_view);

        let Some(compensation_curve) = world
            .resource::<RenderAssets<AutoExposureCompensationCurve>>()
            .get(&auto_exposure.compensation_curve)
        else {
            return Ok(());
        };

        let mut uniform = encase::UniformBuffer::new(Vec::new());
        uniform.write(&auto_exposure.uniform).unwrap();
        let uniform =
            render_context
                .render_device()
                .create_buffer_with_data(&BufferInitDescriptor {
                    label: None,
                    contents: uniform.as_ref(),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

        let compute_bind_group = render_context.render_device().create_bind_group(
            None,
            &pipeline.histogram_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(source),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(mask),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&compensation_curve.texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: compensation_curve.extents.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: resources.histogram.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: auto_exposure.state.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: view_uniforms_buffer,
                        size: Some(ViewUniform::min_size()),
                        offset: 0,
                    }),
                },
            ],
        );

        let mut compute_pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("auto_exposure_pass"),
                    timestamp_writes: None,
                });

        compute_pass.set_bind_group(0, &compute_bind_group, &[view_uniform_offset.offset]);
        compute_pass.set_pipeline(histogram_pipeline);
        compute_pass.dispatch_workgroups(
            (view.viewport.z + 15) / 16,
            (view.viewport.w + 15) / 16,
            1,
        );
        compute_pass.set_pipeline(average_pipeline);
        compute_pass.dispatch_workgroups(1, 1, 1);

        Ok(())
    }
}
