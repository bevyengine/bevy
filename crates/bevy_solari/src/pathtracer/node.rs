use super::{prepare::PathtracerAccumulationTexture, Pathtracer};
use crate::scene::{AtmosphereBindings, RaytracingSceneBindings};
use bevy_asset::{load_embedded_asset, AssetServer};
use bevy_ecs::{prelude::*, resource::Resource, system::Commands};
use bevy_pbr::resources::{AtmosphereTextures, GpuAtmosphere};
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{
        binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor,
        ImageSubresourceRange, PipelineCache, SamplerBindingType, ShaderStages,
        StorageTextureAccess, TextureFormat, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice, ViewQuery},
    uniform::DynamicUniformIndex,
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_utils::default;

/// Resource holding the pathtracer pipeline configuration.
#[derive(Resource)]
pub struct PathtracerPipelines {
    bind_group_layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
}

/// Initializes the pathtracer pipelines at render startup.
pub fn init_pathtracer_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    scene_bindings: Res<RaytracingSceneBindings>,
    asset_server: Res<AssetServer>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "pathtracer_bind_group_layout",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::COMPUTE,
            (
                (
                    0,
                    texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::ReadWrite),
                ),
                (
                    1,
                    texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                ),
                (2, uniform_buffer::<ViewUniform>(true)),
                (
                    10,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (11, sampler(SamplerBindingType::Filtering)),
                (12, uniform_buffer::<GpuAtmosphere>(true)),
            ),
        ),
    );

    let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("pathtracer_pipeline".into()),
        layout: vec![
            scene_bindings.bind_group_layout.clone(),
            bind_group_layout.clone(),
        ],
        shader: load_embedded_asset!(asset_server.as_ref(), "pathtracer.wesl"),
        ..default()
    });

    commands.insert_resource(PathtracerPipelines {
        bind_group_layout,
        pipeline,
    });
}

pub fn pathtracer(
    view: ViewQuery<(
        &Pathtracer,
        &PathtracerAccumulationTexture,
        &ExtractedCamera,
        &ViewTarget,
        &ViewUniformOffset,
        Option<&AtmosphereTextures>,
        Option<&DynamicUniformIndex<GpuAtmosphere>>,
    )>,
    pathtracer_pipelines: Option<Res<PathtracerPipelines>>,
    pipeline_cache: Res<PipelineCache>,
    scene_bindings: Res<RaytracingSceneBindings>,
    view_uniforms: Res<ViewUniforms>,
    atmosphere_bindings: AtmosphereBindings,
    render_device: Res<RenderDevice>,
    mut ctx: RenderContext,
) {
    let (
        pathtracer_settings,
        accumulation_texture,
        camera,
        view_target,
        view_uniform_offset,
        atmosphere_textures,
        atmosphere_uniform_index,
    ) = view.into_inner();

    let Some(pathtracer_pipelines) = pathtracer_pipelines else {
        return;
    };

    let (Some(pipeline), Some(scene_bind_group), Some(viewport), Some(view_uniforms_binding)) = (
        pipeline_cache.get_compute_pipeline(pathtracer_pipelines.pipeline),
        &scene_bindings.bind_group,
        camera.physical_viewport_size,
        view_uniforms.uniforms.binding(),
    ) else {
        return;
    };

    let atmosphere = atmosphere_bindings.view(atmosphere_textures, atmosphere_uniform_index);

    let bind_group = render_device.create_bind_group(
        "pathtracer_bind_group",
        &pipeline_cache.get_bind_group_layout(&pathtracer_pipelines.bind_group_layout),
        &BindGroupEntries::with_indices((
            (0, &accumulation_texture.0.default_view),
            (1, view_target.get_unsampled_color_attachment().view),
            (2, view_uniforms_binding),
            (10, atmosphere.transmittance_lut),
            (11, atmosphere.sampler),
            (12, atmosphere.uniform),
        )),
    );

    let command_encoder = ctx.command_encoder();

    if pathtracer_settings.reset {
        command_encoder.clear_texture(
            &accumulation_texture.0.texture,
            &ImageSubresourceRange::default(),
        );
    }

    let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("pathtracer"),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, scene_bind_group, &[]);
    pass.set_bind_group(
        1,
        &bind_group,
        &[view_uniform_offset.offset, atmosphere.offset],
    );
    pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);
}
