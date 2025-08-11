use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::{Camera, Camera3d, Projection};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prepass::{DepthPrepass, MotionVectorPrepass, ViewPrepassTextures},
    FullscreenShader,
};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    prelude::{Component, Entity, ReflectComponent},
    query::{QueryItem, With},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::World,
};
use bevy_image::{BevyDefault as _, ToExtents};
use bevy_math::vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::{ExtractedCamera, MipBias, TemporalJitter},
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{sampler, texture_2d, texture_depth_2d},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FilterMode, FragmentState, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderStages, SpecializedRenderPipeline,
        SpecializedRenderPipelines, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages,
    },
    renderer::{RenderContext, RenderDevice},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, Msaa, ViewTarget},
    ExtractSchedule, MainWorld, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use bevy_utils::default;
use tracing::warn;

/// Plugin for temporal anti-aliasing.
///
/// See [`TemporalAntiAliasing`] for more details.
pub struct TemporalAntiAliasPlugin;

impl Plugin for TemporalAntiAliasPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "taa.wgsl");

        app.add_plugins(SyncComponentPlugin::<TemporalAntiAliasing>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<TaaPipeline>>()
            .add_systems(RenderStartup, init_taa_pipeline)
            .add_systems(ExtractSchedule, extract_taa_settings)
            .add_systems(
                Render,
                (
                    prepare_taa_jitter.in_set(RenderSystems::ManageViews),
                    prepare_taa_pipelines.in_set(RenderSystems::Prepare),
                    prepare_taa_history_textures.in_set(RenderSystems::PrepareResources),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<TemporalAntiAliasNode>>(Core3d, Node3d::Taa)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    Node3d::MotionBlur, // Running before TAA reduces edge artifacts and noise
                    Node3d::Taa,
                    Node3d::Bloom,
                    Node3d::Tonemapping,
                ),
            );
    }
}

/// Component to apply temporal anti-aliasing to a 3D perspective camera.
///
/// Temporal anti-aliasing (TAA) is a form of image smoothing/filtering, like
/// multisample anti-aliasing (MSAA), or fast approximate anti-aliasing (FXAA).
/// TAA works by blending (averaging) each frame with the past few frames.
///
/// # Tradeoffs
///
/// Pros:
/// * Filters more types of aliasing than MSAA, such as textures and singular bright pixels (specular aliasing)
/// * Cost scales with screen/view resolution, unlike MSAA which scales with number of triangles
/// * Greatly increases the quality of stochastic rendering techniques such as SSAO, certain shadow map sampling methods, etc
///
/// Cons:
/// * Chance of "ghosting" - ghostly trails left behind moving objects
/// * Thin geometry, lighting detail, or texture lines may flicker noisily or disappear
///
/// Because TAA blends past frames with the current frame, when the frames differ too much
/// (such as with fast moving objects or camera cuts), ghosting artifacts may occur.
///
/// Artifacts tend to be reduced at higher framerates and rendering resolution.
///
/// # Usage Notes
///
/// Any camera with this component must also disable [`Msaa`] by setting it to [`Msaa::Off`].
///
/// [Currently](https://github.com/bevyengine/bevy/issues/8423), TAA cannot be used with [`bevy_camera::OrthographicProjection`].
///
/// TAA also does not work well with alpha-blended meshes, as it requires depth writing to determine motion.
///
/// It is very important that correct motion vectors are written for everything on screen.
/// Failure to do so will lead to ghosting artifacts. For instance, if particle effects
/// are added using a third party library, the library must either:
///
/// 1. Write particle motion vectors to the motion vectors prepass texture
/// 2. Render particles after TAA
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(TemporalJitter, MipBias, DepthPrepass, MotionVectorPrepass)]
#[doc(alias = "Taa")]
pub struct TemporalAntiAliasing {
    /// Set to true to delete the saved temporal history (past frames).
    ///
    /// Useful for preventing ghosting when the history is no longer
    /// representative of the current frame, such as in sudden camera cuts.
    ///
    /// After setting this to true, it will automatically be toggled
    /// back to false at the end of the frame.
    pub reset: bool,
}

impl Default for TemporalAntiAliasing {
    fn default() -> Self {
        Self { reset: true }
    }
}

/// Render [`bevy_render::render_graph::Node`] used by temporal anti-aliasing.
#[derive(Default)]
pub struct TemporalAntiAliasNode;

impl ViewNode for TemporalAntiAliasNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static TemporalAntiAliasHistoryTextures,
        &'static ViewPrepassTextures,
        &'static TemporalAntiAliasPipelineId,
        &'static Msaa,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, view_target, taa_history_textures, prepass_textures, taa_pipeline_id, msaa): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if *msaa != Msaa::Off {
            warn!("Temporal anti-aliasing requires MSAA to be disabled");
            return Ok(());
        }

        let (Some(pipelines), Some(pipeline_cache)) = (
            world.get_resource::<TaaPipeline>(),
            world.get_resource::<PipelineCache>(),
        ) else {
            return Ok(());
        };
        let (Some(taa_pipeline), Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) = (
            pipeline_cache.get_render_pipeline(taa_pipeline_id.0),
            &prepass_textures.motion_vectors,
            &prepass_textures.depth,
        ) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let view_target = view_target.post_process_write();

        let taa_bind_group = render_context.render_device().create_bind_group(
            "taa_bind_group",
            &pipelines.taa_bind_group_layout,
            &BindGroupEntries::sequential((
                view_target.source,
                &taa_history_textures.read.default_view,
                &prepass_motion_vectors_texture.texture.default_view,
                &prepass_depth_texture.texture.default_view,
                &pipelines.nearest_sampler,
                &pipelines.linear_sampler,
            )),
        );

        {
            let mut taa_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("taa"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: view_target.destination,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                    Some(RenderPassColorAttachment {
                        view: &taa_history_textures.write.default_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let pass_span = diagnostics.pass_span(&mut taa_pass, "taa");

            taa_pass.set_render_pipeline(taa_pipeline);
            taa_pass.set_bind_group(0, &taa_bind_group, &[]);
            if let Some(viewport) = camera.viewport.as_ref() {
                taa_pass.set_camera_viewport(viewport);
            }
            taa_pass.draw(0..3, 0..1);

            pass_span.end(&mut taa_pass);
        }

        Ok(())
    }
}

#[derive(Resource)]
struct TaaPipeline {
    taa_bind_group_layout: BindGroupLayout,
    nearest_sampler: Sampler,
    linear_sampler: Sampler,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

fn init_taa_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("taa_nearest_sampler"),
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        ..SamplerDescriptor::default()
    });
    let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("taa_linear_sampler"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..SamplerDescriptor::default()
    });

    let taa_bind_group_layout = render_device.create_bind_group_layout(
        "taa_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // View target (read)
                texture_2d(TextureSampleType::Float { filterable: true }),
                // TAA History (read)
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Motion Vectors
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Depth
                texture_depth_2d(),
                // Nearest sampler
                sampler(SamplerBindingType::NonFiltering),
                // Linear sampler
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    commands.insert_resource(TaaPipeline {
        taa_bind_group_layout,
        nearest_sampler,
        linear_sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "taa.wgsl"),
    });
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct TaaPipelineKey {
    hdr: bool,
    reset: bool,
}

impl SpecializedRenderPipeline for TaaPipeline {
    type Key = TaaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];

        let format = if key.hdr {
            shader_defs.push("TONEMAP".into());
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        if key.reset {
            shader_defs.push("RESET".into());
        }

        RenderPipelineDescriptor {
            label: Some("taa_pipeline".into()),
            layout: vec![self.taa_bind_group_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                ],
                ..default()
            }),
            ..default()
        }
    }
}

fn extract_taa_settings(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d = main_world.query::<(
        RenderEntity,
        &Camera,
        &Projection,
        Option<&mut TemporalAntiAliasing>,
    )>();

    for (entity, camera, camera_projection, taa_settings) in cameras_3d.iter_mut(&mut main_world) {
        let has_perspective_projection = matches!(camera_projection, Projection::Perspective(_));
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if let Some(mut taa_settings) = taa_settings
            && camera.is_active
            && has_perspective_projection
        {
            entity_commands.insert(taa_settings.clone());
            taa_settings.reset = false;
        } else {
            entity_commands.remove::<(
                TemporalAntiAliasing,
                TemporalAntiAliasHistoryTextures,
                TemporalAntiAliasPipelineId,
            )>();
        }
    }
}

fn prepare_taa_jitter(
    frame_count: Res<FrameCount>,
    mut query: Query<
        &mut TemporalJitter,
        (
            With<TemporalAntiAliasing>,
            With<Camera3d>,
            With<TemporalJitter>,
            With<DepthPrepass>,
            With<MotionVectorPrepass>,
        ),
    >,
) {
    // Halton sequence (2, 3) - 0.5
    let halton_sequence = [
        vec2(0.0, 0.0),
        vec2(0.0, -0.16666666),
        vec2(-0.25, 0.16666669),
        vec2(0.25, -0.3888889),
        vec2(-0.375, -0.055555552),
        vec2(0.125, 0.2777778),
        vec2(-0.125, -0.2777778),
        vec2(0.375, 0.055555582),
    ];

    let offset = halton_sequence[frame_count.0 as usize % halton_sequence.len()];

    for mut jitter in &mut query {
        jitter.offset = offset;
    }
}

#[derive(Component)]
pub struct TemporalAntiAliasHistoryTextures {
    write: CachedTexture,
    read: CachedTexture,
}

fn prepare_taa_history_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView), With<TemporalAntiAliasing>>,
) {
    for (entity, camera, view) in &views {
        if let Some(physical_target_size) = camera.physical_target_size {
            let mut texture_descriptor = TextureDescriptor {
                label: None,
                size: physical_target_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            texture_descriptor.label = Some("taa_history_1_texture");
            let history_1_texture = texture_cache.get(&render_device, texture_descriptor.clone());

            texture_descriptor.label = Some("taa_history_2_texture");
            let history_2_texture = texture_cache.get(&render_device, texture_descriptor);

            let textures = if frame_count.0 % 2 == 0 {
                TemporalAntiAliasHistoryTextures {
                    write: history_1_texture,
                    read: history_2_texture,
                }
            } else {
                TemporalAntiAliasHistoryTextures {
                    write: history_2_texture,
                    read: history_1_texture,
                }
            };

            commands.entity(entity).insert(textures);
        }
    }
}

#[derive(Component)]
pub struct TemporalAntiAliasPipelineId(CachedRenderPipelineId);

fn prepare_taa_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TaaPipeline>>,
    pipeline: Res<TaaPipeline>,
    views: Query<(Entity, &ExtractedView, &TemporalAntiAliasing)>,
) {
    for (entity, view, taa_settings) in &views {
        let mut pipeline_key = TaaPipelineKey {
            hdr: view.hdr,
            reset: taa_settings.reset,
        };
        let pipeline_id = pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key.clone());

        // Prepare non-reset pipeline anyways - it will be necessary next frame
        if pipeline_key.reset {
            pipeline_key.reset = false;
            pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key);
        }

        commands
            .entity(entity)
            .insert(TemporalAntiAliasPipelineId(pipeline_id));
    }
}
