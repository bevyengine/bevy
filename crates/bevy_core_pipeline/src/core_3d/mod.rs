mod camera_3d;
mod main_opaque_pass_3d_node;
mod main_transmissive_pass_3d_node;
mod main_transparent_pass_3d_node;

pub mod graph {
    pub const NAME: &str = "core_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const MSAA_WRITEBACK: &str = "msaa_writeback";
        pub const PREPASS: &str = "prepass";
        pub const DEFERRED_PREPASS: &str = "deferred_prepass";
        pub const COPY_DEFERRED_LIGHTING_ID: &str = "copy_deferred_lighting_id";
        pub const END_PREPASSES: &str = "end_prepasses";
        pub const START_MAIN_PASS: &str = "start_main_pass";
        pub const MAIN_OPAQUE_PASS: &str = "main_opaque_pass";
        pub const MAIN_TRANSMISSIVE_PASS: &str = "main_transmissive_pass";
        pub const MAIN_TRANSPARENT_PASS: &str = "main_transparent_pass";
        pub const END_MAIN_PASS: &str = "end_main_pass";
        pub const BLOOM: &str = "bloom";
        pub const TONEMAPPING: &str = "tonemapping";
        pub const FXAA: &str = "fxaa";
        pub const UPSCALING: &str = "upscaling";
        pub const CONTRAST_ADAPTIVE_SHARPENING: &str = "contrast_adaptive_sharpening";
        pub const END_MAIN_PASS_POST_PROCESSING: &str = "end_main_pass_post_processing";
    }
}
pub const CORE_3D: &str = graph::NAME;

// PERF: vulkan docs recommend using 24 bit depth for better performance
pub const CORE_3D_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

use std::{cmp::Reverse, ops::Range};

pub use camera_3d::*;
pub use main_opaque_pass_3d_node::*;
pub use main_transparent_pass_3d_node::*;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::{Camera, ExtractedCamera},
    extract_component::ExtractComponentPlugin,
    prelude::Msaa,
    render_graph::{EmptyNode, RenderGraphApp, ViewNodeRunner},
    render_phase::{
        sort_phase_system, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
        RenderPhase,
    },
    render_resource::{
        CachedRenderPipelineId, Extent3d, FilterMode, Sampler, SamplerDescriptor, Texture,
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    },
    renderer::RenderDevice,
    texture::{BevyDefault, TextureCache},
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::{nonmax::NonMaxU32, tracing::warn, FloatOrd, HashMap};

use crate::{
    core_3d::main_transmissive_pass_3d_node::MainTransmissivePass3dNode,
    deferred::{
        copy_lighting_id::CopyDeferredLightingIdNode, node::DeferredGBufferPrepassNode,
        AlphaMask3dDeferred, Opaque3dDeferred, DEFERRED_LIGHTING_PASS_ID_FORMAT,
        DEFERRED_PREPASS_FORMAT,
    },
    prepass::{
        node::PrepassNode, AlphaMask3dPrepass, DeferredPrepass, DepthPrepass, MotionVectorPrepass,
        NormalPrepass, Opaque3dPrepass, ViewPrepassTextures, MOTION_VECTOR_PREPASS_FORMAT,
        NORMAL_PREPASS_FORMAT,
    },
    skybox::SkyboxPlugin,
    tonemapping::TonemappingNode,
    upscaling::UpscalingNode,
};

pub struct Core3dPlugin;

impl Plugin for Core3dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera3d>()
            .register_type::<Camera3dDepthLoadOp>()
            .add_plugins((SkyboxPlugin, ExtractComponentPlugin::<Camera3d>::default()))
            .add_systems(PostUpdate, check_msaa);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DrawFunctions<Opaque3d>>()
            .init_resource::<DrawFunctions<AlphaMask3d>>()
            .init_resource::<DrawFunctions<Transmissive3d>>()
            .init_resource::<DrawFunctions<Transparent3d>>()
            .init_resource::<DrawFunctions<Opaque3dPrepass>>()
            .init_resource::<DrawFunctions<AlphaMask3dPrepass>>()
            .init_resource::<DrawFunctions<Opaque3dDeferred>>()
            .init_resource::<DrawFunctions<AlphaMask3dDeferred>>()
            .add_systems(ExtractSchedule, extract_core_3d_camera_phases)
            .add_systems(ExtractSchedule, extract_camera_prepass_phase)
            .add_systems(
                Render,
                (
                    sort_phase_system::<Opaque3d>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<AlphaMask3d>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<Transmissive3d>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<Transparent3d>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<Opaque3dPrepass>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<AlphaMask3dPrepass>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<Opaque3dDeferred>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<AlphaMask3dDeferred>.in_set(RenderSet::PhaseSort),
                    prepare_core_3d_depth_textures.in_set(RenderSet::PrepareResources),
                    prepare_core_3d_transmission_textures.in_set(RenderSet::PrepareResources),
                    prepare_prepass_textures.in_set(RenderSet::PrepareResources),
                ),
            );

        use graph::node::*;
        render_app
            .add_render_sub_graph(CORE_3D)
            .add_render_graph_node::<ViewNodeRunner<PrepassNode>>(CORE_3D, PREPASS)
            .add_render_graph_node::<ViewNodeRunner<DeferredGBufferPrepassNode>>(
                CORE_3D,
                DEFERRED_PREPASS,
            )
            .add_render_graph_node::<ViewNodeRunner<CopyDeferredLightingIdNode>>(
                CORE_3D,
                COPY_DEFERRED_LIGHTING_ID,
            )
            .add_render_graph_node::<EmptyNode>(CORE_3D, END_PREPASSES)
            .add_render_graph_node::<EmptyNode>(CORE_3D, START_MAIN_PASS)
            .add_render_graph_node::<ViewNodeRunner<MainOpaquePass3dNode>>(
                CORE_3D,
                MAIN_OPAQUE_PASS,
            )
            .add_render_graph_node::<ViewNodeRunner<MainTransmissivePass3dNode>>(
                CORE_3D,
                MAIN_TRANSMISSIVE_PASS,
            )
            .add_render_graph_node::<ViewNodeRunner<MainTransparentPass3dNode>>(
                CORE_3D,
                MAIN_TRANSPARENT_PASS,
            )
            .add_render_graph_node::<EmptyNode>(CORE_3D, END_MAIN_PASS)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(CORE_3D, TONEMAPPING)
            .add_render_graph_node::<EmptyNode>(CORE_3D, END_MAIN_PASS_POST_PROCESSING)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(CORE_3D, UPSCALING)
            .add_render_graph_edges(
                CORE_3D,
                &[
                    PREPASS,
                    DEFERRED_PREPASS,
                    COPY_DEFERRED_LIGHTING_ID,
                    END_PREPASSES,
                    START_MAIN_PASS,
                    MAIN_OPAQUE_PASS,
                    MAIN_TRANSMISSIVE_PASS,
                    MAIN_TRANSPARENT_PASS,
                    END_MAIN_PASS,
                    TONEMAPPING,
                    END_MAIN_PASS_POST_PROCESSING,
                    UPSCALING,
                ],
            );
    }
}

pub struct Opaque3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub dynamic_offset: Option<NonMaxU32>,
}

impl PhaseItem for Opaque3d {
    // NOTE: Values increase towards the camera. Front-to-back ordering for opaque means we need a descending sort.
    type SortKey = Reverse<FloatOrd>;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        Reverse(FloatOrd(self.distance))
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // Key negated to match reversed SortKey ordering
        radsort::sort_by_key(items, |item| -item.distance);
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn dynamic_offset(&self) -> Option<NonMaxU32> {
        self.dynamic_offset
    }

    #[inline]
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32> {
        &mut self.dynamic_offset
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct AlphaMask3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub dynamic_offset: Option<NonMaxU32>,
}

impl PhaseItem for AlphaMask3d {
    // NOTE: Values increase towards the camera. Front-to-back ordering for alpha mask means we need a descending sort.
    type SortKey = Reverse<FloatOrd>;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        Reverse(FloatOrd(self.distance))
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // Key negated to match reversed SortKey ordering
        radsort::sort_by_key(items, |item| -item.distance);
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn dynamic_offset(&self) -> Option<NonMaxU32> {
        self.dynamic_offset
    }

    #[inline]
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32> {
        &mut self.dynamic_offset
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct Transmissive3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub dynamic_offset: Option<NonMaxU32>,
}

impl PhaseItem for Transmissive3d {
    // NOTE: Values increase towards the camera. Back-to-front ordering for transmissive means we need an ascending sort.
    type SortKey = FloatOrd;

    /// For now, automatic batching is disabled for transmissive items because their rendering is
    /// split into multiple steps depending on [`Camera3d::screen_space_specular_transmission_steps`],
    /// which the batching system doesn't currently know about.
    ///
    /// Having batching enabled would cause the same item to be drawn multiple times across different
    /// steps, whenever the batching range crossed a step boundary.
    ///
    /// Eventually, we could add support for this by having the batching system break up the batch ranges
    /// using the same logic as the transmissive pass, but for now it's simpler to just disable batching.
    const AUTOMATIC_BATCHING: bool = false;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn dynamic_offset(&self) -> Option<NonMaxU32> {
        self.dynamic_offset
    }

    #[inline]
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32> {
        &mut self.dynamic_offset
    }
}

impl CachedRenderPipelinePhaseItem for Transmissive3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct Transparent3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub dynamic_offset: Option<NonMaxU32>,
}

impl PhaseItem for Transparent3d {
    // NOTE: Values increase towards the camera. Back-to-front ordering for transparent means we need an ascending sort.
    type SortKey = FloatOrd;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn dynamic_offset(&self) -> Option<NonMaxU32> {
        self.dynamic_offset
    }

    #[inline]
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32> {
        &mut self.dynamic_offset
    }
}

impl CachedRenderPipelinePhaseItem for Transparent3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_core_3d_camera_phases(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    for (entity, camera) in &cameras_3d {
        if camera.is_active {
            commands.get_or_spawn(entity).insert((
                RenderPhase::<Opaque3d>::default(),
                RenderPhase::<AlphaMask3d>::default(),
                RenderPhase::<Transmissive3d>::default(),
                RenderPhase::<Transparent3d>::default(),
            ));
        }
    }
}

// Extract the render phases for the prepass
pub fn extract_camera_prepass_phase(
    mut commands: Commands,
    cameras_3d: Extract<
        Query<
            (
                Entity,
                &Camera,
                Has<DepthPrepass>,
                Has<NormalPrepass>,
                Has<MotionVectorPrepass>,
                Has<DeferredPrepass>,
            ),
            With<Camera3d>,
        >,
    >,
) {
    for (entity, camera, depth_prepass, normal_prepass, motion_vector_prepass, deferred_prepass) in
        cameras_3d.iter()
    {
        if camera.is_active {
            let mut entity = commands.get_or_spawn(entity);

            if depth_prepass || normal_prepass || motion_vector_prepass {
                entity.insert((
                    RenderPhase::<Opaque3dPrepass>::default(),
                    RenderPhase::<AlphaMask3dPrepass>::default(),
                ));
            }

            if deferred_prepass {
                entity.insert((
                    RenderPhase::<Opaque3dDeferred>::default(),
                    RenderPhase::<AlphaMask3dDeferred>::default(),
                ));
            }

            if depth_prepass {
                entity.insert(DepthPrepass);
            }
            if normal_prepass {
                entity.insert(NormalPrepass);
            }
            if motion_vector_prepass {
                entity.insert(MotionVectorPrepass);
            }
            if deferred_prepass {
                entity.insert(DeferredPrepass);
            }
        }
    }
}

pub fn prepare_core_3d_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (Entity, &ExtractedCamera, Option<&DepthPrepass>, &Camera3d),
        (
            With<RenderPhase<Opaque3d>>,
            With<RenderPhase<AlphaMask3d>>,
            With<RenderPhase<Transmissive3d>>,
            With<RenderPhase<Transparent3d>>,
        ),
    >,
) {
    let mut render_target_usage = HashMap::default();
    for (_, camera, depth_prepass, camera_3d) in &views_3d {
        // Default usage required to write to the depth texture
        let mut usage: TextureUsages = camera_3d.depth_texture_usages.into();
        if depth_prepass.is_some() {
            // Required to read the output of the prepass
            usage |= TextureUsages::COPY_SRC;
        }
        render_target_usage
            .entry(camera.target.clone())
            .and_modify(|u| *u |= usage)
            .or_insert_with(|| usage);
    }

    let mut textures = HashMap::default();
    for (entity, camera, _, _) in &views_3d {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let cached_texture = textures
            .entry(camera.target.clone())
            .or_insert_with(|| {
                // The size of the depth texture
                let size = Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_target_size.x,
                    height: physical_target_size.y,
                };

                let usage = *render_target_usage
                    .get(&camera.target.clone())
                    .expect("The depth texture usage should already exist for this target");

                let descriptor = TextureDescriptor {
                    label: Some("view_depth_texture"),
                    size,
                    mip_level_count: 1,
                    sample_count: msaa.samples(),
                    dimension: TextureDimension::D2,
                    format: CORE_3D_DEPTH_FORMAT,
                    usage,
                    view_formats: &[],
                };

                texture_cache.get(&render_device, descriptor)
            })
            .clone();

        commands.entity(entity).insert(ViewDepthTexture {
            texture: cached_texture.texture,
            view: cached_texture.default_view,
        });
    }
}

#[derive(Component)]
pub struct ViewTransmissionTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

pub fn prepare_core_3d_transmission_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (
            Entity,
            &ExtractedCamera,
            &Camera3d,
            &ExtractedView,
            &RenderPhase<Transmissive3d>,
        ),
        (
            With<RenderPhase<Opaque3d>>,
            With<RenderPhase<AlphaMask3d>>,
            With<RenderPhase<Transparent3d>>,
        ),
    >,
) {
    let mut textures = HashMap::default();
    for (entity, camera, camera_3d, view, transmissive_3d_phase) in &views_3d {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        // Don't prepare a transmission texture if the number of steps is set to 0
        if camera_3d.screen_space_specular_transmission_steps == 0 {
            continue;
        }

        // Don't prepare a transmission texture if there are no transmissive items to render
        if transmissive_3d_phase.items.is_empty() {
            continue;
        }

        let cached_texture = textures
            .entry(camera.target.clone())
            .or_insert_with(|| {
                let usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;

                // The size of the transmission texture
                let size = Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_target_size.x,
                    height: physical_target_size.y,
                };

                let format = if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                };

                let descriptor = TextureDescriptor {
                    label: Some("view_transmission_texture"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1, // No need for MSAA, as we'll only copy the main texture here
                    dimension: TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                };

                texture_cache.get(&render_device, descriptor)
            })
            .clone();

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("view_transmission_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        commands.entity(entity).insert(ViewTransmissionTexture {
            texture: cached_texture.texture,
            view: cached_texture.default_view,
            sampler,
        });
    }
}

// Disable MSAA and warn if using deferred rendering
pub fn check_msaa(
    mut msaa: ResMut<Msaa>,
    deferred_views: Query<Entity, (With<Camera>, With<DeferredPrepass>)>,
) {
    if !deferred_views.is_empty() {
        match *msaa {
            Msaa::Off => (),
            _ => {
                warn!("MSAA is incompatible with deferred rendering and has been disabled.");
                *msaa = Msaa::Off;
            }
        };
    }
}

// Prepares the textures used by the prepass
pub fn prepare_prepass_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (
            Entity,
            &ExtractedCamera,
            Has<DepthPrepass>,
            Has<NormalPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        Or<(
            With<RenderPhase<Opaque3dPrepass>>,
            With<RenderPhase<AlphaMask3dPrepass>>,
            With<RenderPhase<Opaque3dDeferred>>,
            With<RenderPhase<AlphaMask3dDeferred>>,
        )>,
    >,
) {
    let mut depth_textures = HashMap::default();
    let mut normal_textures = HashMap::default();
    let mut deferred_textures = HashMap::default();
    let mut deferred_lighting_id_textures = HashMap::default();
    let mut motion_vectors_textures = HashMap::default();
    for (entity, camera, depth_prepass, normal_prepass, motion_vector_prepass, deferred_prepass) in
        &views_3d
    {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let size = Extent3d {
            depth_or_array_layers: 1,
            width: physical_target_size.x,
            height: physical_target_size.y,
        };

        let cached_depth_texture = depth_prepass.then(|| {
            depth_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    let descriptor = TextureDescriptor {
                        label: Some("prepass_depth_texture"),
                        size,
                        mip_level_count: 1,
                        sample_count: msaa.samples(),
                        dimension: TextureDimension::D2,
                        format: CORE_3D_DEPTH_FORMAT,
                        usage: TextureUsages::COPY_DST
                            | TextureUsages::RENDER_ATTACHMENT
                            | TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    };
                    texture_cache.get(&render_device, descriptor)
                })
                .clone()
        });

        let cached_normals_texture = normal_prepass.then(|| {
            normal_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("prepass_normal_texture"),
                            size,
                            mip_level_count: 1,
                            sample_count: msaa.samples(),
                            dimension: TextureDimension::D2,
                            format: NORMAL_PREPASS_FORMAT,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                    )
                })
                .clone()
        });

        let cached_motion_vectors_texture = motion_vector_prepass.then(|| {
            motion_vectors_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("prepass_motion_vectors_textures"),
                            size,
                            mip_level_count: 1,
                            sample_count: msaa.samples(),
                            dimension: TextureDimension::D2,
                            format: MOTION_VECTOR_PREPASS_FORMAT,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                    )
                })
                .clone()
        });

        let cached_deferred_texture = deferred_prepass.then(|| {
            deferred_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("prepass_deferred_texture"),
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format: DEFERRED_PREPASS_FORMAT,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                    )
                })
                .clone()
        });

        let deferred_lighting_pass_id_texture = deferred_prepass.then(|| {
            deferred_lighting_id_textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("deferred_lighting_pass_id_texture"),
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format: DEFERRED_LIGHTING_PASS_ID_FORMAT,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                    )
                })
                .clone()
        });

        commands.entity(entity).insert(ViewPrepassTextures {
            depth: cached_depth_texture,
            normal: cached_normals_texture,
            motion_vectors: cached_motion_vectors_texture,
            deferred: cached_deferred_texture,
            deferred_lighting_pass_id: deferred_lighting_pass_id_texture,
            size,
        });
    }
}
