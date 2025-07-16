use super::{prepare::SolariLightingResources, SolariLighting};
use crate::scene::RaytracingSceneBindings;
use bevy_asset::load_embedded_asset;
use bevy_core_pipeline::prepass::{
    PreviousViewData, PreviousViewUniformOffset, PreviousViewUniforms, ViewPrepassTextures,
};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    query::QueryItem,
    world::{FromWorld, World},
};
use bevy_image::ToExtents;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        binding_types::{
            storage_buffer_sized, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
        },
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId,
        ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, PushConstantRange,
        ShaderStages, StorageTextureAccess, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_utils::default;

pub mod graph {
    use bevy_render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub struct SolariLightingNode;
}

pub struct SolariLightingNode {
    bind_group_layout: BindGroupLayout,
    di_initial_and_temporal_pipeline: CachedComputePipelineId,
    di_spatial_and_shade_pipeline: CachedComputePipelineId,
    gi_initial_and_temporal_pipeline: CachedComputePipelineId,
    gi_spatial_and_shade_pipeline: CachedComputePipelineId,
}

impl ViewNode for SolariLightingNode {
    type ViewQuery = (
        &'static SolariLighting,
        &'static SolariLightingResources,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        &'static PreviousViewUniformOffset,
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
            previous_view_uniform_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let previous_view_uniforms = world.resource::<PreviousViewUniforms>();
        let frame_count = world.resource::<FrameCount>();
        let (
            Some(di_initial_and_temporal_pipeline),
            Some(di_spatial_and_shade_pipeline),
            Some(gi_initial_and_temporal_pipeline),
            Some(gi_spatial_and_shade_pipeline),
            Some(scene_bindings),
            Some(viewport),
            Some(gbuffer),
            Some(depth_buffer),
            Some(motion_vectors),
            Some(view_uniforms),
            Some(previous_view_uniforms),
        ) = (
            pipeline_cache.get_compute_pipeline(self.di_initial_and_temporal_pipeline),
            pipeline_cache.get_compute_pipeline(self.di_spatial_and_shade_pipeline),
            pipeline_cache.get_compute_pipeline(self.gi_initial_and_temporal_pipeline),
            pipeline_cache.get_compute_pipeline(self.gi_spatial_and_shade_pipeline),
            &scene_bindings.bind_group,
            camera.physical_viewport_size,
            view_prepass_textures.deferred_view(),
            view_prepass_textures.depth_view(),
            view_prepass_textures.motion_vectors_view(),
            view_uniforms.uniforms.binding(),
            previous_view_uniforms.uniforms.binding(),
        )
        else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            "solari_lighting_bind_group",
            &self.bind_group_layout,
            &BindGroupEntries::sequential((
                view_target.get_unsampled_color_attachment().view,
                solari_lighting_resources
                    .di_reservoirs_a
                    .as_entire_binding(),
                solari_lighting_resources
                    .di_reservoirs_b
                    .as_entire_binding(),
                solari_lighting_resources
                    .gi_reservoirs_a
                    .as_entire_binding(),
                solari_lighting_resources
                    .gi_reservoirs_b
                    .as_entire_binding(),
                gbuffer,
                depth_buffer,
                motion_vectors,
                &solari_lighting_resources.previous_gbuffer.1,
                &solari_lighting_resources.previous_depth.1,
                view_uniforms,
                previous_view_uniforms,
            )),
        );

        // Choice of number here is arbitrary
        let frame_index = frame_count.0.wrapping_mul(5782582);

        let diagnostics = render_context.diagnostic_recorder();
        let command_encoder = render_context.command_encoder();

        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("solari_lighting"),
            timestamp_writes: None,
        });
        let pass_span = diagnostics.pass_span(&mut pass, "solari_lighting");

        pass.set_bind_group(0, scene_bindings, &[]);
        pass.set_bind_group(
            1,
            &bind_group,
            &[
                view_uniform_offset.offset,
                previous_view_uniform_offset.offset,
            ],
        );

        pass.set_pipeline(di_initial_and_temporal_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);

        pass.set_pipeline(di_spatial_and_shade_pipeline);
        pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);

        pass.set_pipeline(gi_initial_and_temporal_pipeline);
        pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);

        pass.set_pipeline(gi_spatial_and_shade_pipeline);
        pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);

        pass_span.end(&mut pass);
        drop(pass);

        // TODO: Remove these copies, and double buffer instead
        command_encoder.copy_texture_to_texture(
            view_prepass_textures
                .deferred
                .clone()
                .unwrap()
                .texture
                .texture
                .as_image_copy(),
            solari_lighting_resources.previous_gbuffer.0.as_image_copy(),
            viewport.to_extents(),
        );
        command_encoder.copy_texture_to_texture(
            view_prepass_textures
                .depth
                .clone()
                .unwrap()
                .texture
                .texture
                .as_image_copy(),
            solari_lighting_resources.previous_depth.0.as_image_copy(),
            viewport.to_extents(),
        );

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
                        StorageTextureAccess::ReadWrite,
                    ),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    texture_2d(TextureSampleType::Uint),
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Uint),
                    texture_depth_2d(),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<PreviousViewData>(true),
                ),
            ),
        );

        let di_initial_and_temporal_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("solari_lighting_di_initial_and_temporal_pipeline".into()),
                layout: vec![
                    scene_bindings.bind_group_layout.clone(),
                    bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: load_embedded_asset!(world, "restir_di.wgsl"),
                entry_point: Some("initial_and_temporal".into()),
                ..default()
            });

        let di_spatial_and_shade_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("solari_lighting_di_spatial_and_shade_pipeline".into()),
                layout: vec![
                    scene_bindings.bind_group_layout.clone(),
                    bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: load_embedded_asset!(world, "restir_di.wgsl"),
                entry_point: Some("spatial_and_shade".into()),
                ..default()
            });

        let gi_initial_and_temporal_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("solari_lighting_gi_initial_and_temporal_pipeline".into()),
                layout: vec![
                    scene_bindings.bind_group_layout.clone(),
                    bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: load_embedded_asset!(world, "restir_gi.wgsl"),
                shader_defs: vec![],
                entry_point: Some("initial_and_temporal".into()),
                zero_initialize_workgroup_memory: false,
            });

        let gi_spatial_and_shade_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("solari_lighting_gi_spatial_and_shade_pipeline".into()),
                layout: vec![
                    scene_bindings.bind_group_layout.clone(),
                    bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader: load_embedded_asset!(world, "restir_gi.wgsl"),
                shader_defs: vec![],
                entry_point: Some("spatial_and_shade".into()),
                zero_initialize_workgroup_memory: false,
            });

        Self {
            bind_group_layout,
            di_initial_and_temporal_pipeline,
            di_spatial_and_shade_pipeline,
            gi_initial_and_temporal_pipeline,
            gi_spatial_and_shade_pipeline,
        }
    }
}
