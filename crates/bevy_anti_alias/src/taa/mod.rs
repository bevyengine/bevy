use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer};
use bevy_camera::{Camera, Camera3d};
use bevy_core_pipeline::{
    prepass::{DepthPrepass, MotionVectorPrepass, ViewPrepassTextures},
    schedule::{Core3d, Core3dSystems},
    tonemapping::tonemapping,
    FullscreenShader,
};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    error::BevyError,
    prelude::{Component, Entity, ReflectComponent},
    query::With,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::{BevyDefault as _, ToExtents};
use bevy_math::vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::{ExtractedCamera, MipBias, TemporalJitter},
    diagnostic::RecordDiagnostics,
    render_resource::{
        binding_types::{sampler, texture_2d, texture_depth_2d},
        BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedRenderPipelineId, Canonical, ColorTargetState, ColorWrites, FilterMode,
        FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
        ShaderStages, Specializer, SpecializerKey, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages, Variants,
    },
    renderer::{RenderContext, RenderDevice, ViewQuery},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, Msaa, ViewTarget},
    ExtractSchedule, MainWorld, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_utils::default;
use tracing::warn;

/// Plugin for temporal anti-aliasing.
///
/// See [`TemporalAntiAliasing`] for more details.
#[derive(Default)]
pub struct TemporalAntiAliasPlugin;

impl Plugin for TemporalAntiAliasPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "taa.wgsl");

        app.add_plugins(SyncComponentPlugin::<TemporalAntiAliasing>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_taa_pipeline)
            .add_systems(ExtractSchedule, extract_taa_settings)
            .add_systems(
                Render,
                (
                    prepare_taa_jitter.in_set(RenderSystems::ManageViews),
                    prepare_taa_pipelines.in_set(RenderSystems::Prepare),
                    prepare_taa_history_textures.in_set(RenderSystems::PrepareResources),
                ),
            );

        render_app.add_systems(
            Core3d,
            temporal_anti_alias
                .before(tonemapping)
                .in_set(Core3dSystems::PostProcess),
        );
    }
}

/// Component to apply temporal anti-aliasing to a 3D camera.
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

fn temporal_anti_alias(
    view: ViewQuery<(
        &ExtractedCamera,
        &ViewTarget,
        &TemporalAntiAliasHistoryTextures,
        &ViewPrepassTextures,
        &TemporalAntiAliasPipelineId,
        &Msaa,
    )>,
    pipelines: Option<Res<TaaPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (camera, view_target, taa_history_textures, prepass_textures, taa_pipeline_id, msaa) =
        view.into_inner();

    if *msaa != Msaa::Off {
        warn!("Temporal anti-aliasing requires MSAA to be disabled");
        return;
    }

    let Some(pipelines) = pipelines else {
        return;
    };
    let (Some(taa_pipeline), Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) = (
        pipeline_cache.get_render_pipeline(taa_pipeline_id.0),
        &prepass_textures.motion_vectors,
        &prepass_textures.depth,
    ) else {
        return;
    };

    let view_target = view_target.post_process_write();

    let taa_bind_group = ctx.render_device().create_bind_group(
        "taa_bind_group",
        &pipeline_cache.get_bind_group_layout(&pipelines.taa_bind_group_layout),
        &BindGroupEntries::sequential((
            view_target.source,
            &taa_history_textures.read.default_view,
            &prepass_motion_vectors_texture.texture.default_view,
            &prepass_depth_texture.texture.default_view,
            &pipelines.nearest_sampler,
            &pipelines.linear_sampler,
        )),
    );

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let mut taa_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
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
        multiview_mask: None,
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

#[derive(Resource)]
struct TaaPipeline {
    taa_bind_group_layout: BindGroupLayoutDescriptor,
    nearest_sampler: Sampler,
    linear_sampler: Sampler,
    variants: Variants<RenderPipeline, TaaPipelineSpecializer>,
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

    let taa_bind_group_layout = BindGroupLayoutDescriptor::new(
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

    let fragment_shader = load_embedded_asset!(asset_server.as_ref(), "taa.wgsl");

    let variants = Variants::new(
        TaaPipelineSpecializer,
        RenderPipelineDescriptor {
            label: Some("taa_pipeline".into()),
            layout: vec![taa_bind_group_layout.clone()],
            vertex: fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: fragment_shader,
                ..default()
            }),
            ..default()
        },
    );

    commands.insert_resource(TaaPipeline {
        taa_bind_group_layout,
        nearest_sampler,
        linear_sampler,
        variants,
    });
}

struct TaaPipelineSpecializer;

#[derive(PartialEq, Eq, Hash, Clone, SpecializerKey)]
struct TaaPipelineKey {
    hdr: bool,
    reset: bool,
}

impl Specializer<RenderPipeline> for TaaPipelineSpecializer {
    type Key = TaaPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        let fragment = descriptor.fragment_mut()?;
        let format = if key.hdr {
            fragment.shader_defs.push("TONEMAP".into());
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        if key.reset {
            fragment.shader_defs.push("RESET".into());
        }

        let color_target_state = ColorTargetState {
            format,
            blend: None,
            write_mask: ColorWrites::ALL,
        };

        fragment.set_target(0, color_target_state.clone());
        fragment.set_target(1, color_target_state);

        Ok(key)
    }
}

fn extract_taa_settings(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d =
        main_world.query::<(RenderEntity, &Camera, Option<&mut TemporalAntiAliasing>)>();

    for (entity, camera, taa_settings) in cameras_3d.iter_mut(&mut main_world) {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if let Some(mut taa_settings) = taa_settings
            && camera.is_active
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

            let textures = if frame_count.0.is_multiple_of(2) {
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
    mut pipeline: ResMut<TaaPipeline>,
    views: Query<(Entity, &ExtractedView, &TemporalAntiAliasing)>,
) -> Result<(), BevyError> {
    for (entity, view, taa_settings) in &views {
        let mut pipeline_key = TaaPipelineKey {
            hdr: view.hdr,
            reset: taa_settings.reset,
        };
        let pipeline_id = pipeline
            .variants
            .specialize(&pipeline_cache, pipeline_key.clone())?;

        // Prepare non-reset pipeline anyways - it will be necessary next frame
        if pipeline_key.reset {
            pipeline_key.reset = false;
            pipeline
                .variants
                .specialize(&pipeline_cache, pipeline_key)?;
        }

        commands
            .entity(entity)
            .insert(TemporalAntiAliasPipelineId(pipeline_id));
    }

    Ok(())
}
