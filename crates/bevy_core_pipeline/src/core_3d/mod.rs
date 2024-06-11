mod camera_3d;
mod main_opaque_pass_3d_node;
mod main_transmissive_pass_3d_node;
mod main_transparent_pass_3d_node;

pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct Core3d;

    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum Node3d {
        MsaaWriteback,
        Prepass,
        DeferredPrepass,
        CopyDeferredLightingId,
        EndPrepasses,
        StartMainPass,
        MainOpaquePass,
        MainTransmissivePass,
        MainTransparentPass,
        EndMainPass,
        Taa,
        MotionBlur,
        Bloom,
        AutoExposure,
        DepthOfField,
        Tonemapping,
        Fxaa,
        Smaa,
        Upscaling,
        ContrastAdaptiveSharpening,
        EndMainPassPostProcessing,
    }
}

// PERF: vulkan docs recommend using 24 bit depth for better performance
pub const CORE_3D_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// True if multisampled depth textures are supported on this platform.
///
/// In theory, Naga supports depth textures on WebGL 2. In practice, it doesn't,
/// because of a silly bug whereby Naga assumes that all depth textures are
/// `sampler2DShadow` and will cheerfully generate invalid GLSL that tries to
/// perform non-percentage-closer-filtering with such a sampler. Therefore we
/// disable depth of field and screen space reflections entirely on WebGL 2.
#[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
pub const DEPTH_TEXTURE_SAMPLING_SUPPORTED: bool = false;

/// True if multisampled depth textures are supported on this platform.
///
/// In theory, Naga supports depth textures on WebGL 2. In practice, it doesn't,
/// because of a silly bug whereby Naga assumes that all depth textures are
/// `sampler2DShadow` and will cheerfully generate invalid GLSL that tries to
/// perform non-percentage-closer-filtering with such a sampler. Therefore we
/// disable depth of field and screen space reflections entirely on WebGL 2.
#[cfg(any(feature = "webgpu", not(target_arch = "wasm32")))]
pub const DEPTH_TEXTURE_SAMPLING_SUPPORTED: bool = true;

use std::ops::Range;

use bevy_asset::AssetId;
use bevy_color::LinearRgba;
pub use camera_3d::*;
pub use main_opaque_pass_3d_node::*;
pub use main_transparent_pass_3d_node::*;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{entity::EntityHashSet, prelude::*};
use bevy_math::FloatOrd;
use bevy_render::{
    camera::{Camera, ExtractedCamera},
    extract_component::ExtractComponentPlugin,
    mesh::Mesh,
    prelude::Msaa,
    render_graph::{EmptyNode, RenderGraphApp, ViewNodeRunner},
    render_phase::{
        sort_phase_system, BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId,
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, SortedPhaseItem, ViewBinnedRenderPhases,
        ViewSortedRenderPhases,
    },
    render_resource::{
        BindGroupId, CachedRenderPipelineId, Extent3d, FilterMode, Sampler, SamplerDescriptor,
        Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    },
    renderer::RenderDevice,
    texture::{BevyDefault, ColorAttachment, Image, TextureCache},
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::{tracing::warn, HashMap};

use crate::{
    core_3d::main_transmissive_pass_3d_node::MainTransmissivePass3dNode,
    deferred::{
        copy_lighting_id::CopyDeferredLightingIdNode, node::DeferredGBufferPrepassNode,
        AlphaMask3dDeferred, Opaque3dDeferred, DEFERRED_LIGHTING_PASS_ID_FORMAT,
        DEFERRED_PREPASS_FORMAT,
    },
    dof::DepthOfFieldNode,
    prepass::{
        node::PrepassNode, AlphaMask3dPrepass, DeferredPrepass, DepthPrepass, MotionVectorPrepass,
        NormalPrepass, Opaque3dPrepass, OpaqueNoLightmap3dBinKey, ViewPrepassTextures,
        MOTION_VECTOR_PREPASS_FORMAT, NORMAL_PREPASS_FORMAT,
    },
    skybox::SkyboxPlugin,
    tonemapping::TonemappingNode,
    upscaling::UpscalingNode,
};

use self::graph::{Core3d, Node3d};

pub struct Core3dPlugin;

impl Plugin for Core3dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera3d>()
            .register_type::<ScreenSpaceTransmissionQuality>()
            .add_plugins((SkyboxPlugin, ExtractComponentPlugin::<Camera3d>::default()))
            .add_systems(PostUpdate, check_msaa);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
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
                    sort_phase_system::<Transmissive3d>.in_set(RenderSet::PhaseSort),
                    sort_phase_system::<Transparent3d>.in_set(RenderSet::PhaseSort),
                    prepare_core_3d_depth_textures.in_set(RenderSet::PrepareResources),
                    prepare_core_3d_transmission_textures.in_set(RenderSet::PrepareResources),
                    prepare_prepass_textures.in_set(RenderSet::PrepareResources),
                ),
            );

        render_app
            .add_render_sub_graph(Core3d)
            .add_render_graph_node::<ViewNodeRunner<PrepassNode>>(Core3d, Node3d::Prepass)
            .add_render_graph_node::<ViewNodeRunner<DeferredGBufferPrepassNode>>(
                Core3d,
                Node3d::DeferredPrepass,
            )
            .add_render_graph_node::<ViewNodeRunner<CopyDeferredLightingIdNode>>(
                Core3d,
                Node3d::CopyDeferredLightingId,
            )
            .add_render_graph_node::<EmptyNode>(Core3d, Node3d::EndPrepasses)
            .add_render_graph_node::<EmptyNode>(Core3d, Node3d::StartMainPass)
            .add_render_graph_node::<ViewNodeRunner<MainOpaquePass3dNode>>(
                Core3d,
                Node3d::MainOpaquePass,
            )
            .add_render_graph_node::<ViewNodeRunner<MainTransmissivePass3dNode>>(
                Core3d,
                Node3d::MainTransmissivePass,
            )
            .add_render_graph_node::<ViewNodeRunner<MainTransparentPass3dNode>>(
                Core3d,
                Node3d::MainTransparentPass,
            )
            .add_render_graph_node::<EmptyNode>(Core3d, Node3d::EndMainPass)
            .add_render_graph_node::<ViewNodeRunner<DepthOfFieldNode>>(Core3d, Node3d::DepthOfField)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(Core3d, Node3d::Tonemapping)
            .add_render_graph_node::<EmptyNode>(Core3d, Node3d::EndMainPassPostProcessing)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(Core3d, Node3d::Upscaling)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Prepass,
                    Node3d::DeferredPrepass,
                    Node3d::CopyDeferredLightingId,
                    Node3d::EndPrepasses,
                    Node3d::StartMainPass,
                    Node3d::MainOpaquePass,
                    Node3d::MainTransmissivePass,
                    Node3d::MainTransparentPass,
                    Node3d::EndMainPass,
                    Node3d::Tonemapping,
                    Node3d::EndMainPassPostProcessing,
                    Node3d::Upscaling,
                ),
            );
    }
}

/// Opaque 3D [`BinnedPhaseItem`]s.
pub struct Opaque3d {
    /// The key, which determines which can be batched.
    pub key: Opaque3dBinKey,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub representative_entity: Entity,
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

/// Data that must be identical in order to batch meshes together.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opaque3dBinKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,

    /// The function used to draw.
    pub draw_function: DrawFunctionId,

    /// The mesh.
    pub asset_id: AssetId<Mesh>,

    /// The ID of a bind group specific to the material.
    ///
    /// In the case of PBR, this is the `MaterialBindGroupId`.
    pub material_bind_group_id: Option<BindGroupId>,

    /// The lightmap, if present.
    pub lightmap_image: Option<AssetId<Image>>,
}

impl PhaseItem for Opaque3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.key.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for Opaque3d {
    type BinKey = Opaque3dBinKey;

    #[inline]
    fn new(
        key: Self::BinKey,
        representative_entity: Entity,
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Opaque3d {
            key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.key.pipeline
    }
}

pub struct AlphaMask3d {
    pub key: OpaqueNoLightmap3dBinKey,
    pub representative_entity: Entity,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for AlphaMask3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.key.draw_function
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
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for AlphaMask3d {
    type BinKey = OpaqueNoLightmap3dBinKey;

    #[inline]
    fn new(
        key: Self::BinKey,
        representative_entity: Entity,
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.key.pipeline
    }
}

pub struct Transmissive3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for Transmissive3d {
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
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
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
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for Transmissive3d {
    // NOTE: Values increase towards the camera. Back-to-front ordering for transmissive means we need an ascending sort.
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
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
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for Transparent3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
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
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for Transparent3d {
    // NOTE: Values increase towards the camera. Back-to-front ordering for transparent means we need an ascending sort.
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
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
    mut opaque_3d_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut alpha_mask_3d_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3d>>,
    mut transmissive_3d_phases: ResMut<ViewSortedRenderPhases<Transmissive3d>>,
    mut transparent_3d_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();

    for (entity, camera) in &cameras_3d {
        if !camera.is_active {
            continue;
        }

        commands.get_or_spawn(entity);

        opaque_3d_phases.insert_or_clear(entity);
        alpha_mask_3d_phases.insert_or_clear(entity);
        transmissive_3d_phases.insert_or_clear(entity);
        transparent_3d_phases.insert_or_clear(entity);

        live_entities.insert(entity);
    }

    opaque_3d_phases.retain(|entity, _| live_entities.contains(entity));
    alpha_mask_3d_phases.retain(|entity, _| live_entities.contains(entity));
    transmissive_3d_phases.retain(|entity, _| live_entities.contains(entity));
    transparent_3d_phases.retain(|entity, _| live_entities.contains(entity));
}

// Extract the render phases for the prepass
pub fn extract_camera_prepass_phase(
    mut commands: Commands,
    mut opaque_3d_prepass_phases: ResMut<ViewBinnedRenderPhases<Opaque3dPrepass>>,
    mut alpha_mask_3d_prepass_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3dPrepass>>,
    mut opaque_3d_deferred_phases: ResMut<ViewBinnedRenderPhases<Opaque3dDeferred>>,
    mut alpha_mask_3d_deferred_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3dDeferred>>,
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
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();

    for (entity, camera, depth_prepass, normal_prepass, motion_vector_prepass, deferred_prepass) in
        cameras_3d.iter()
    {
        if !camera.is_active {
            continue;
        }

        if depth_prepass || normal_prepass || motion_vector_prepass {
            opaque_3d_prepass_phases.insert_or_clear(entity);
            alpha_mask_3d_prepass_phases.insert_or_clear(entity);
        } else {
            opaque_3d_prepass_phases.remove(&entity);
            alpha_mask_3d_prepass_phases.remove(&entity);
        }

        if deferred_prepass {
            opaque_3d_deferred_phases.insert_or_clear(entity);
            alpha_mask_3d_deferred_phases.insert_or_clear(entity);
        } else {
            opaque_3d_deferred_phases.remove(&entity);
            alpha_mask_3d_deferred_phases.remove(&entity);
        }

        live_entities.insert(entity);

        let mut entity = commands.get_or_spawn(entity);

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

    opaque_3d_prepass_phases.retain(|entity, _| live_entities.contains(entity));
    alpha_mask_3d_prepass_phases.retain(|entity, _| live_entities.contains(entity));
    opaque_3d_deferred_phases.retain(|entity, _| live_entities.contains(entity));
    alpha_mask_3d_deferred_phases.retain(|entity, _| live_entities.contains(entity));
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_core_3d_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    opaque_3d_phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
    alpha_mask_3d_phases: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    transmissive_3d_phases: Res<ViewSortedRenderPhases<Transmissive3d>>,
    transparent_3d_phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    views_3d: Query<(Entity, &ExtractedCamera, Option<&DepthPrepass>, &Camera3d)>,
) {
    let mut render_target_usage = HashMap::default();
    for (view, camera, depth_prepass, camera_3d) in &views_3d {
        if !opaque_3d_phases.contains_key(&view)
            || !alpha_mask_3d_phases.contains_key(&view)
            || !transmissive_3d_phases.contains_key(&view)
            || !transparent_3d_phases.contains_key(&view)
        {
            continue;
        };

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
    for (entity, camera, _, camera_3d) in &views_3d {
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

        commands.entity(entity).insert(ViewDepthTexture::new(
            cached_texture,
            match camera_3d.depth_load_op {
                Camera3dDepthLoadOp::Clear(v) => Some(v),
                Camera3dDepthLoadOp::Load => None,
            },
        ));
    }
}

#[derive(Component)]
pub struct ViewTransmissionTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_core_3d_transmission_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    opaque_3d_phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
    alpha_mask_3d_phases: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    transmissive_3d_phases: Res<ViewSortedRenderPhases<Transmissive3d>>,
    transparent_3d_phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    views_3d: Query<(Entity, &ExtractedCamera, &Camera3d, &ExtractedView)>,
) {
    let mut textures = HashMap::default();
    for (entity, camera, camera_3d, view) in &views_3d {
        if !opaque_3d_phases.contains_key(&entity)
            || !alpha_mask_3d_phases.contains_key(&entity)
            || !transparent_3d_phases.contains_key(&entity)
        {
            continue;
        };

        let Some(transmissive_3d_phase) = transmissive_3d_phases.get(&entity) else {
            continue;
        };

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
#[allow(clippy::too_many_arguments)]
pub fn prepare_prepass_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    opaque_3d_prepass_phases: Res<ViewBinnedRenderPhases<Opaque3dPrepass>>,
    alpha_mask_3d_prepass_phases: Res<ViewBinnedRenderPhases<AlphaMask3dPrepass>>,
    opaque_3d_deferred_phases: Res<ViewBinnedRenderPhases<Opaque3dDeferred>>,
    alpha_mask_3d_deferred_phases: Res<ViewBinnedRenderPhases<AlphaMask3dDeferred>>,
    views_3d: Query<(
        Entity,
        &ExtractedCamera,
        Has<DepthPrepass>,
        Has<NormalPrepass>,
        Has<MotionVectorPrepass>,
        Has<DeferredPrepass>,
    )>,
) {
    let mut depth_textures = HashMap::default();
    let mut normal_textures = HashMap::default();
    let mut deferred_textures = HashMap::default();
    let mut deferred_lighting_id_textures = HashMap::default();
    let mut motion_vectors_textures = HashMap::default();
    for (entity, camera, depth_prepass, normal_prepass, motion_vector_prepass, deferred_prepass) in
        &views_3d
    {
        if !opaque_3d_prepass_phases.contains_key(&entity)
            && !alpha_mask_3d_prepass_phases.contains_key(&entity)
            && !opaque_3d_deferred_phases.contains_key(&entity)
            && !alpha_mask_3d_deferred_phases.contains_key(&entity)
        {
            continue;
        };

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

        let cached_deferred_lighting_pass_id_texture = deferred_prepass.then(|| {
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
            depth: cached_depth_texture
                .map(|t| ColorAttachment::new(t, None, Some(LinearRgba::BLACK))),
            normal: cached_normals_texture
                .map(|t| ColorAttachment::new(t, None, Some(LinearRgba::BLACK))),
            // Red and Green channels are X and Y components of the motion vectors
            // Blue channel doesn't matter, but set to 0.0 for possible faster clear
            // https://gpuopen.com/performance/#clears
            motion_vectors: cached_motion_vectors_texture
                .map(|t| ColorAttachment::new(t, None, Some(LinearRgba::BLACK))),
            deferred: cached_deferred_texture
                .map(|t| ColorAttachment::new(t, None, Some(LinearRgba::BLACK))),
            deferred_lighting_pass_id: cached_deferred_lighting_pass_id_texture
                .map(|t| ColorAttachment::new(t, None, Some(LinearRgba::BLACK))),
            size,
        });
    }
}
