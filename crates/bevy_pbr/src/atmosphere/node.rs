use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_math::{UVec2, Vec3Swizzles};
use bevy_render::{
    extract_component::DynamicUniformIndex,
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::{
        ComputePass, ComputePassDescriptor, Extent3d, Origin3d, PipelineCache,
        RenderPassDescriptor, TextureAspect,
    },
    renderer::RenderContext,
    texture::GpuImage,
    view::{ViewTarget, ViewUniformOffset},
};

use crate::ViewLightsUniformOffset;

use super::{
    resources::{
        AtmosphereBindGroups, AtmosphereLutPipelines, AtmosphereResources,
        AtmosphereTransformsOffset, EnvironmentPipeline, RenderSkyPipelineId,
    },
    Atmosphere, AtmosphereSettings,
};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum AtmosphereNode {
    RenderLuts,
    RenderSky,
    Environment,
}

#[derive(Default)]
pub(super) struct AtmosphereLutsNode {}

impl ViewNode for AtmosphereLutsNode {
    type ViewQuery = (
        Read<AtmosphereSettings>,
        Read<AtmosphereBindGroups>,
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            settings,
            bind_groups,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<AtmosphereLutPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (
            Some(transmittance_lut_pipeline),
            Some(multiscattering_lut_pipeline),
            Some(sky_view_lut_pipeline),
            Some(aerial_view_lut_pipeline),
        ) = (
            pipeline_cache.get_compute_pipeline(pipelines.transmittance_lut),
            pipeline_cache.get_compute_pipeline(pipelines.multiscattering_lut),
            pipeline_cache.get_compute_pipeline(pipelines.sky_view_lut),
            pipeline_cache.get_compute_pipeline(pipelines.aerial_view_lut),
        )
        else {
            return Ok(());
        };

        let command_encoder = render_context.command_encoder();

        let mut luts_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("atmosphere_luts_pass"),
            timestamp_writes: None,
        });

        fn dispatch_2d(compute_pass: &mut ComputePass, size: UVec2) {
            const WORKGROUP_SIZE: u32 = 16;
            let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
            let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Transmittance LUT

        luts_pass.set_pipeline(transmittance_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.transmittance_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
            ],
        );

        dispatch_2d(&mut luts_pass, settings.transmittance_lut_size);

        // Multiscattering LUT

        luts_pass.set_pipeline(multiscattering_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.multiscattering_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
            ],
        );

        luts_pass.dispatch_workgroups(
            settings.multiscattering_lut_size.x,
            settings.multiscattering_lut_size.y,
            1,
        );

        // Sky View LUT

        luts_pass.set_pipeline(sky_view_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.sky_view_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );

        dispatch_2d(&mut luts_pass, settings.sky_view_lut_size);

        // Aerial View LUT

        luts_pass.set_pipeline(aerial_view_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.aerial_view_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );

        dispatch_2d(&mut luts_pass, settings.aerial_view_lut_size.xy());

        Ok(())
    }
}

#[derive(Default)]
pub(super) struct RenderSkyNode;

impl ViewNode for RenderSkyNode {
    type ViewQuery = (
        Read<AtmosphereBindGroups>,
        Read<ViewTarget>,
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<RenderSkyPipelineId>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            atmosphere_bind_groups,
            view_target,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            render_sky_pipeline_id,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_sky_pipeline) =
            pipeline_cache.get_render_pipeline(render_sky_pipeline_id.0)
        else {
            return Ok(());
        }; //TODO: warning

        let mut render_sky_pass =
            render_context
                .command_encoder()
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("render_sky_pass"),
                    color_attachments: &[Some(view_target.get_color_attachment())],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

        render_sky_pass.set_pipeline(render_sky_pipeline);
        render_sky_pass.set_bind_group(
            0,
            &atmosphere_bind_groups.render_sky,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );
        render_sky_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Default)]
pub(super) struct EnvironmentNode;

impl ViewNode for EnvironmentNode {
    type ViewQuery = (
        Read<AtmosphereBindGroups>,
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            bind_groups,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<AtmosphereLutPipelines>();
        let gpu_images = world.resource::<RenderAssets<GpuImage>>();
        let atmosphere_resources = world.resource::<AtmosphereResources>();

        // Get the texture for the environment array from the resources
        let Some(environment_specular_storage) =
            gpu_images.get(&atmosphere_resources.environment_specular_storage_view)
        else {
            return Ok(());
        };

        // Get the cubemap texture view as well
        let Some(environment_specular) = gpu_images.get(&atmosphere_resources.environment_specular)
        else {
            return Ok(());
        };

        // Get the diffuse irradiance map
        let Some(environment_diffuse_storage) =
            gpu_images.get(&atmosphere_resources.environment_diffuse_storage_view)
        else {
            return Ok(());
        };

        // Get the diffuse irradiance map
        let Some(environment_diffuse) = gpu_images.get(&atmosphere_resources.environment_diffuse)
        else {
            return Ok(());
        };

        // Get the base level pipeline for environment cubemap generation
        let Some(specular_pipeline) =
            pipeline_cache.get_compute_pipeline(pipelines.environment_specular)
        else {
            return Ok(());
        };

        // Get the diffuse irradiance pipeline
        let Some(diffuse_pipeline) =
            pipeline_cache.get_compute_pipeline(pipelines.environment_diffuse)
        else {
            return Ok(());
        };

        // Generate the environment cubemap (specular reflections)
        {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("environment_specular_pass"),
                        timestamp_writes: None,
                    });

            pass.set_pipeline(specular_pipeline);
            pass.set_bind_group(
                0,
                &bind_groups.environment,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                    atmosphere_transforms_offset.index(),
                    view_uniforms_offset.offset,
                    lights_uniforms_offset.offset,
                ],
            );

            pass.dispatch_workgroups(
                environment_specular_storage.size.width / 8,
                environment_specular_storage.size.height / 8,
                6, // 6 cubemap faces
            );
        }

        // Generate the diffuse irradiance map
        {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("diffuse_irradiance_pass"),
                        timestamp_writes: None,
                    });

            pass.set_pipeline(diffuse_pipeline);
            pass.set_bind_group(
                0,
                &bind_groups.environment,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                    atmosphere_transforms_offset.index(),
                    view_uniforms_offset.offset,
                    lights_uniforms_offset.offset,
                ],
            );

            pass.dispatch_workgroups(
                environment_diffuse.size.width / 8,
                environment_diffuse.size.height / 8,
                6, // 6 cubemap faces
            );
        }

        // Copy environment cubemap to the final texture
        render_context.command_encoder().copy_texture_to_texture(
            environment_specular_storage.texture.as_image_copy(),
            environment_specular.texture.as_image_copy(),
            Extent3d {
                width: environment_specular_storage.size.width,
                height: environment_specular_storage.size.height,
                depth_or_array_layers: 6,
            },
        );

        // Copy diffuse irradiance map to the diffuse cubemap
        render_context.command_encoder().copy_texture_to_texture(
            environment_diffuse_storage.texture.as_image_copy(),
            environment_diffuse.texture.as_image_copy(),
            Extent3d {
                width: environment_diffuse.size.width,
                height: environment_diffuse.size.height,
                depth_or_array_layers: 6,
            },
        );

        Ok(())
    }
}
