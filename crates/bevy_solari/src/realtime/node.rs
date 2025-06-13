use super::{prepare::SolariLightingResources, SolariLighting};
use crate::scene::RaytracingSceneBindings;
use bevy_asset::load_embedded_asset;
use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    query::QueryItem,
    world::{FromWorld, World},
};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        binding_types::{
            storage_buffer_read_only_sized, storage_buffer_sized, texture_2d, texture_depth_2d,
            texture_storage_2d, uniform_buffer,
        },
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId,
        ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, PushConstantRange,
        ShaderStages, StorageTextureAccess, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};

pub mod graph {
    use bevy_render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub struct SolariLightingNode;
}

pub struct SolariLightingNode {
    bind_group_layout: BindGroupLayout,
    initial_samples_pipeline: CachedComputePipelineId,
    spatial_reuse_pipeline: CachedComputePipelineId,
}

impl ViewNode for SolariLightingNode {
    type ViewQuery = (
        &'static SolariLighting,
        &'static SolariLightingResources,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            solari_lighting,
            solari_lighting_resources,
            camera,
            view_target,
            view_prepass_textures,
            view_uniform_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let frame_count = world.resource::<FrameCount>();
        let (
            Some(initial_samples_pipeline),
            Some(spatial_reuse_pipeline),
            Some(scene_bindings),
            Some(viewport),
            Some(gbuffer),
            Some(depth_buffer),
            Some(motion_vectors),
            Some(view_uniforms),
        ) = (
            pipeline_cache.get_compute_pipeline(self.initial_samples_pipeline),
            pipeline_cache.get_compute_pipeline(self.spatial_reuse_pipeline),
            &scene_bindings.bind_group,
            camera.physical_viewport_size,
            view_prepass_textures.deferred_view(),
            view_prepass_textures.depth_view(),
            view_prepass_textures.motion_vectors_view(),
            view_uniforms.uniforms.binding(),
        )
        else {
            return Ok(());
        };

        let (reservoirs, previous_reservoirs) = if frame_count.0 % 2 == 0 {
            (
                &solari_lighting_resources.reservoirs_a,
                &solari_lighting_resources.reservoirs_b,
            )
        } else {
            (
                &solari_lighting_resources.reservoirs_b,
                &solari_lighting_resources.reservoirs_a,
            )
        };

        let bind_group = render_context.render_device().create_bind_group(
            "solari_lighting_bind_group",
            &self.bind_group_layout,
            &BindGroupEntries::sequential((
                view_target.get_unsampled_color_attachment().view,
                previous_reservoirs.as_entire_binding(),
                reservoirs.as_entire_binding(),
                gbuffer,
                depth_buffer,
                motion_vectors,
                view_uniforms,
            )),
        );

        let frame_index = frame_count.0.wrapping_mul(5782582);
        let command_encoder = render_context.command_encoder();

        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("solari_lighting"),
            timestamp_writes: None,
        });
        pass.set_bind_group(0, scene_bindings, &[]);
        pass.set_bind_group(1, &bind_group, &[view_uniform_offset.offset]);

        pass.set_pipeline(initial_samples_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);

        pass.set_pipeline(spatial_reuse_pipeline);
        pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);

        Ok(())
    }
}

impl FromWorld for SolariLightingNode {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "solari_lighting_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(
                        ViewTarget::TEXTURE_FORMAT_HDR,
                        StorageTextureAccess::WriteOnly,
                    ),
                    storage_buffer_read_only_sized(false, None),
                    storage_buffer_sized(false, None),
                    texture_2d(TextureSampleType::Uint),
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        let initial_samples_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("solari_lighting_initial_samples_pipeline".into()),
                layout: vec![
                    scene_bindings.bind_group_layout.clone(),
                    bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: load_embedded_asset!(world, "direct.wgsl"),
                shader_defs: vec![],
                entry_point: "initial_samples".into(),
                zero_initialize_workgroup_memory: false,
            });

        let spatial_reuse_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("solari_lighting_spatial_reuse_pipeline".into()),
                layout: vec![
                    scene_bindings.bind_group_layout.clone(),
                    bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: load_embedded_asset!(world, "direct.wgsl"),
                shader_defs: vec![],
                entry_point: "spatial_reuse".into(),
                zero_initialize_workgroup_memory: false,
            });

        Self {
            bind_group_layout,
            initial_samples_pipeline,
            spatial_reuse_pipeline,
        }
    }
}
