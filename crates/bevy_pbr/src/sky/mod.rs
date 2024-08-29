use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetServer, Handle};
use bevy_core_pipeline::core_3d::graph::Node3d;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::Reflect;
use bevy_render::render_resource::binding_types::uniform_buffer;
use bevy_render::render_resource::{
    BindGroupLayoutEntries, CachedComputePipelineId, ComputePipelineDescriptor,
};
use bevy_render::{
    camera::{Camera, ExtractedCamera},
    globals::GlobalsBuffer,
    render_graph::{RenderGraphApp, ViewNode, ViewNodeRunner},
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupLayout, CachedRenderPipelineId, ColorTargetState,
        ColorWrites, Extent3d, FragmentState, MultisampleState, Operations, PipelineCache,
        PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
        Shader, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    renderer::{RenderAdapter, RenderDevice},
    texture::{CachedTexture, TextureCache},
    view::{ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_render::{
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel},
    renderer::RenderContext,
};
use bevy_utils::{prelude::default, tracing::warn};

use bevy_core_pipeline::{
    core_3d::{graph::Core3d, Camera3d},
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SkyLabel;

const TRANSMITTANCE_LUT_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(165991536981278682488488481343101998581);
const COMMON_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(165991536981278682488488481343101998582);
const ATMOSPHERE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(165991536981278682488488481343101998583);

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TRANSMITTANCE_LUT_SHADER_HANDLE,
            "sky_transmittance_lut.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            COMMON_SHADER_HANDLE,
            "sky_common.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            ATMOSPHERE_SHADER_HANDLE,
            "sky_atmosphere.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Sky>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if !render_app
            .world()
            .resource::<RenderAdapter>()
            .get_texture_format_features(TextureFormat::Rgba16Float)
            .allowed_usages
            .contains(TextureUsages::STORAGE_BINDING)
        {
            warn!("SkyPlugin not loaded. GPU lacks support: TextureFormat::Rgba16Float does not support TextureUsages::STORAGE_BINDING.");
            return;
        }

        render_app
            .init_resource::<SkyPipelines>()
            // .init_resource::<SpecializedComputePipelines<SkyPipelines>>()
            .add_systems(ExtractSchedule, extract_sky_settings)
            .add_systems(
                Render,
                (
                    prepare_sky_textures.in_set(RenderSet::PrepareResources),
                    prepare_sky_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<SkyNode>>(Core3d, SkyLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    // END_PRE_PASSES -> PREPARE_SKY -> MAIN_PASS
                    Node3d::EndPrepasses,
                    SkyLabel,
                    Node3d::StartMainPass,
                ),
            );
    }
}

#[derive(Clone, Component, Default, Reflect)]
pub struct Sky {}

fn extract_sky_settings(
    mut commands: Commands,
    cameras: Extract<Query<(Entity, &Camera, &Sky), With<Camera3d>>>,
) {
    for (entity, camera, sky_settings) in &cameras {
        if camera.is_active {
            commands.get_or_spawn(entity).insert(sky_settings.clone());
        }
    }
}

#[derive(Resource)]
struct SkyPipelines {
    transmittance_lut_pipeline: CachedRenderPipelineId,
    // sky_view_lut: CachedRenderPipelineId,
    // aerial_view_lut: CachedRenderPipelineId,
    // multiscattering_lut: CachedComputePipelineId,

    // common_bind_group_layout: BindGroupLayout,
    transmittance_lut_bind_group_layout: BindGroupLayout,
    // sky_view_lut_bind_group_layout: BindGroupLayout,
    // aerial_view_lut_bind_group_layout: BindGroupLayout,
    // multiscattering_lut_bind_group_layout: BindGroupLayout,
}

impl FromWorld for SkyPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let transmittance_lut_bind_group_layout =
            render_device.create_bind_group_layout("sky_transmittance_lut_bind_group_layout", &[]);

        let transmittance_lut_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("sky_transmittance_lut_pipeline".into()),
                layout: vec![transmittance_lut_bind_group_layout.clone()],
                push_constant_ranges: vec![],
                vertex: fullscreen_shader_vertex_state(),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    shader: TRANSMITTANCE_LUT_SHADER_HANDLE.clone(),
                    shader_defs: vec![],
                    entry_point: "main".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
            });

        // let multi_scattering_lut_bind_group_layout = render_device.create_bind_group_layout(
        //     "sky_multi_scattering_lut_bind_group_layout",
        //     &BindGroupLayoutEntries::sequential(
        //         ShaderStages::COMPUTE,
        //         (uniform_buffer::<ViewUniform>(true),),
        //     ),
        // );

        // let multi_scattering_lut_pipeline =
        //     pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        //         label: Some("sky_multi_scattering_lut_pipeline".into()),
        //         layout: vec![multi_scattering_lut_bind_group_layout.clone()],
        //         push_constant_ranges: vec![],
        //         shader: world
        //             .resource::<AssetServer>()
        //             .load("shaders/sky/multi_scattering_lut.wgsl"),
        //         shader_defs: vec![],
        //         entry_point: "main".into(),
        //     });

        Self {
            transmittance_lut_pipeline,
            // sky_view_lut: todo!(),
            // aerial_view_lut: todo!(),
            // multiscattering_lut: todo!(),
            // common_bind_group_layout: todo!(),
            transmittance_lut_bind_group_layout,
            // sky_view_lut_bind_group_layout: todo!(),
            // aerial_view_lut_bind_group_layout: todo!(),
            // multiscattering_lut_bind_group_layout: todo!(),
        }
    }
}

// #[derive(Component)]
// struct TransmittanceLutPipelineId(CachedRenderPipelineId);

// #[derive(Component)]
// struct SkyViewLutPipelineId(CachedRenderPipelineId);

// #[derive(Component)]
// struct AerialPerspectivePipelineId(CachedRenderPipelineId);

// #[derive(Component)]
// struct MultiScatteringLutPipelineId(CachedComputePipelineId);

// fn prepare_ssao_pipelines(
//     mut commands: Commands,
//     pipeline_cache: Res<PipelineCache>,
//     mut render_pipelines: ResMut<SpecializedRenderPipelines<SkyPipelines>>,
//     mut compute_pipelines: ResMut<SpecializedComputePipelines<SkyPipelines>>,
//     pipeline: Res<SkyPipeline>,
//     views: Query<(
//         Entity,
//         // &ScreenSpaceAmbientOcclusionSettings,
//         // Option<&TemporalJitter>,
//     )>,
// ) {
//     for (entity, ssao_settings, temporal_jitter) in &views {
//         let pipeline_id = compute_pipelines.specialize(
//             &pipeline_cache,
//             &pipeline,
//             SsaoPipelineKey {
//                 ssao_settings: ssao_settings.clone(),
//                 temporal_noise: temporal_jitter.is_some(),
//             },
//         );

//         commands.entity(entity).insert(SsaoPipelineId(pipeline_id));
//     }
// }

#[derive(Component)]
struct SkyTextures {
    transmittance_lut: CachedTexture,
    sky_view_lut: CachedTexture,
    aerial_perspective_lut: CachedTexture,
    multi_scattering_lut: CachedTexture,
}

fn prepare_sky_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<Sky>>,
) {
    for (entity, camera) in &views {
        let transmittance_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("transmittance_lut"),
                size: Extent3d {
                    width: 256,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let sky_view_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("sky_view_lut"),
                size: Extent3d {
                    width: 192,
                    height: 108,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rg11b10Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let aerial_perspective_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("aerial_perspective"),
                size: Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let multi_scattering_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("multi_scattering"),
                size: Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert({
            SkyTextures {
                transmittance_lut,
                sky_view_lut,
                aerial_perspective_lut,
                multi_scattering_lut,
            }
        });
    }
}

#[derive(Component)]
struct SkyBindGroups {
    transmittance_lut_bind_group: BindGroup,
}

// Separate prepare needed, because Resources like ViewUniforms are not available in ViewNode::run()
fn prepare_sky_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<SkyPipelines>,
    view_uniforms: Res<ViewUniforms>,
    global_uniforms: Res<GlobalsBuffer>,
    views: Query<(Entity, &SkyTextures)>,
) {
    // bevy_log::debug!("prepare_sky_bindgroups");

    let (Some(view_uniforms), Some(globals_uniforms)) = (
        view_uniforms.uniforms.binding(),
        global_uniforms.buffer.binding(),
    ) else {
        return;
    };

    for (entity, sky_textures) in &views {
        // bevy_log::debug!("{:?}", entity);
        let transmittance_lut_bind_group = render_device.create_bind_group(
            "transmittance_lut_bind_group",
            &pipelines.transmittance_lut_bind_group_layout,
            &[],
        );

        // let multi_scattering_lut_bind_group = render_device.create_bind_group(
        //     "multi_scattering_lut_bind_group",
        //     &pipelines.transmittance_lut_bind_group_layout,
        //     &BindGroupEntries::sequential((view_uniforms.clone(),)),
        // );

        commands.entity(entity).insert(SkyBindGroups {
            transmittance_lut_bind_group,
        });
    }
}

#[derive(Default)]
struct SkyNode {}

impl ViewNode for SkyNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static SkyTextures,
        // &'static SsaoPipelineId,
        &'static SkyBindGroups,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, textures, bind_groups, view_uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<SkyPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (
            //     Some(camera_size),
            Some(transmittance_lut_pipeline),
            //     Some(spatial_denoise_pipeline),
            //     Some(gtao_pipeline),
        ) = (
            //     camera.physical_viewport_size,
            pipeline_cache.get_render_pipeline(pipelines.transmittance_lut_pipeline),
            //     pipeline_cache.get_compute_pipeline(pipelines.spatial_denoise_pipeline),
            //     pipeline_cache.get_compute_pipeline(pipeline_id.0),
        )
        else {
            return Ok(());
        };

        render_context.command_encoder().push_debug_group("sky");

        {
            let mut transmittance_lut_pass =
                render_context
                    .command_encoder()
                    .begin_render_pass(&RenderPassDescriptor {
                        label: Some("transmittance_lut_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &textures.transmittance_lut.default_view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                        ..default()
                    });
            transmittance_lut_pass.set_pipeline(transmittance_lut_pipeline);
            transmittance_lut_pass.set_bind_group(
                0,
                &bind_groups.transmittance_lut_bind_group,
                &[],
            );
            transmittance_lut_pass.draw(0..3, 0..1);
        }

        // {
        //     let mut preprocess_depth_pass =
        //         render_context
        //             .command_encoder()
        //             .begin_compute_pass(&ComputePassDescriptor {
        //                 label: Some("ssao_preprocess_depth_pass"),
        //             });
        //     preprocess_depth_pass.set_pipeline(preprocess_depth_pipeline);
        //     preprocess_depth_pass.set_bind_group(0, &bind_groups.preprocess_depth_bind_group, &[]);
        //     preprocess_depth_pass.set_bind_group(
        //         1,
        //         &bind_groups.common_bind_group,
        //         &[view_uniform_offset.offset],
        //     );
        //     preprocess_depth_pass.dispatch_workgroups(
        //         div_ceil(camera_size.x, 16),
        //         div_ceil(camera_size.y, 16),
        //         1,
        //     );
        // }

        render_context.command_encoder().pop_debug_group();
        Ok(())
    }
}
