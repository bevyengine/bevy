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
        EarlyPrepass,
        EarlyDownsampleDepth,
        LatePrepass,
        EarlyDeferredPrepass,
        LateDeferredPrepass,
        CopyDeferredLightingId,
        EndPrepasses,
        StartMainPass,
        MainOpaquePass,
        MainTransmissivePass,
        MainTransparentPass,
        EndMainPass,
        Wireframe,
        StartMainPassPostProcessing,
        LateDownsampleDepth,
        MotionBlur,
        Taa,
        DlssSuperResolution,
        DlssRayReconstruction,
        Bloom,
        AutoExposure,
        DepthOfField,
        PostProcessing,
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
#[cfg(not(any(feature = "webgpu", not(target_arch = "wasm32"))))]
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

use core::ops::Range;

use bevy_camera::{Camera, Camera3d, Camera3dDepthLoadOp};
use bevy_diagnostic::FrameCount;
use bevy_render::{
    camera::CameraRenderGraph,
    experimental::occlusion_culling::OcclusionCulling,
    mesh::allocator::SlabId,
    render_phase::{BinnedRenderPhase, PhaseItemBatchSetKey, SortedRenderPhase},
    texture::CachedTexture,
    view::{prepare_view_targets, NoIndirectDrawing},
};
pub use main_opaque_pass_3d_node::*;
pub use main_transparent_pass_3d_node::*;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::UntypedAssetId;
use bevy_color::LinearRgba;
use bevy_ecs::prelude::*;
use bevy_image::{BevyDefault, ToExtents};
use bevy_math::FloatOrd;
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::ExtractComponentPlugin,
    prelude::Msaa,
    render_graph::{EmptyNode, RenderGraphExt, ViewNodeRunner},
    render_phase::{
        sort_phase_system, BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId,
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, SortedPhaseItem,
    },
    render_resource::{
        CachedRenderPipelineId, FilterMode, Sampler, SamplerDescriptor, Texture, TextureDescriptor,
        TextureDimension, TextureFormat, TextureUsages, TextureView,
    },
    renderer::RenderDevice,
    sync_world::MainEntity,
    texture::{ColorAttachment, TextureCache},
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
    Render, RenderApp, RenderSystems,
};
use nonmax::NonMaxU32;
use tracing::warn;

use crate::{
    core_3d::main_transmissive_pass_3d_node::MainTransmissivePass3dNode,
    deferred::{
        copy_lighting_id::CopyDeferredLightingIdNode,
        node::{EarlyDeferredGBufferPrepassNode, LateDeferredGBufferPrepassNode},
        AlphaMask3dDeferred, Opaque3dDeferred, DEFERRED_LIGHTING_PASS_ID_FORMAT,
        DEFERRED_PREPASS_FORMAT,
    },
    prepass::{
        node::{EarlyPrepassNode, LatePrepassNode},
        AlphaMask3dPrepass, DeferredPrepass, DeferredPrepassDoubleBuffer, DepthPrepass,
        DepthPrepassDoubleBuffer, MotionVectorPrepass, NormalPrepass, Opaque3dPrepass,
        OpaqueNoLightmap3dBatchSetKey, OpaqueNoLightmap3dBinKey, ViewPrepassTextures,
        MOTION_VECTOR_PREPASS_FORMAT, NORMAL_PREPASS_FORMAT,
    },
    skybox::SkyboxPlugin,
    tonemapping::{DebandDither, Tonemapping, TonemappingNode},
    upscaling::UpscalingNode,
};

use self::graph::{Core3d, Node3d};

pub struct Core3dPlugin;

impl Plugin for Core3dPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components_with::<Camera3d, DebandDither>(|| DebandDither::Enabled)
            .register_required_components_with::<Camera3d, CameraRenderGraph>(|| {
                CameraRenderGraph::new(Core3d)
            })
            .register_required_components::<Camera3d, Tonemapping>()
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
            .add_systems(
                Render,
                (
                    sort_phase_system::<Transmissive3d>.in_set(RenderSystems::PhaseSort),
                    sort_phase_system::<Transparent3d>.in_set(RenderSystems::PhaseSort),
                    configure_occlusion_culling_view_targets
                        .after(prepare_view_targets)
                        .in_set(RenderSystems::ManageViews),
                    prepare_core_3d_depth_textures.in_set(RenderSystems::PrepareResources),
                    prepare_core_3d_transmission_textures.in_set(RenderSystems::PrepareResources),
                    prepare_prepass_textures.in_set(RenderSystems::PrepareResources),
                ),
            );

        render_app
            .add_render_sub_graph(Core3d)
            .add_render_graph_node::<ViewNodeRunner<EarlyPrepassNode>>(Core3d, Node3d::EarlyPrepass)
            .add_render_graph_node::<ViewNodeRunner<LatePrepassNode>>(Core3d, Node3d::LatePrepass)
            .add_render_graph_node::<ViewNodeRunner<EarlyDeferredGBufferPrepassNode>>(
                Core3d,
                Node3d::EarlyDeferredPrepass,
            )
            .add_render_graph_node::<ViewNodeRunner<LateDeferredGBufferPrepassNode>>(
                Core3d,
                Node3d::LateDeferredPrepass,
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
            .add_render_graph_node::<EmptyNode>(Core3d, Node3d::StartMainPassPostProcessing)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(Core3d, Node3d::Tonemapping)
            .add_render_graph_node::<EmptyNode>(Core3d, Node3d::EndMainPassPostProcessing)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(Core3d, Node3d::Upscaling)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EarlyPrepass,
                    Node3d::EarlyDeferredPrepass,
                    Node3d::LatePrepass,
                    Node3d::LateDeferredPrepass,
                    Node3d::CopyDeferredLightingId,
                    Node3d::EndPrepasses,
                    Node3d::StartMainPass,
                    Node3d::MainOpaquePass,
                    Node3d::MainTransmissivePass,
                    Node3d::MainTransparentPass,
                    Node3d::EndMainPass,
                    Node3d::StartMainPassPostProcessing,
                    Node3d::Tonemapping,
                    Node3d::EndMainPassPostProcessing,
                    Node3d::Upscaling,
                ),
            );
    }
}

/// Opaque 3D [`BinnedPhaseItem`]s.
pub struct Opaque3d {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: Opaque3dBatchSetKey,
    /// The key, which determines which can be batched.
    pub bin_key: Opaque3dBinKey,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub representative_entity: (Entity, MainEntity),
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

/// Information that must be identical in order to place opaque meshes in the
/// same *batch set*.
///
/// A batch set is a set of batches that can be multi-drawn together, if
/// multi-draw is in use.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opaque3dBatchSetKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,

    /// The function used to draw.
    pub draw_function: DrawFunctionId,

    /// The ID of a bind group specific to the material instance.
    ///
    /// In the case of PBR, this is the `MaterialBindGroupIndex`.
    pub material_bind_group_index: Option<u32>,

    /// The ID of the slab of GPU memory that contains vertex data.
    ///
    /// For non-mesh items, you can fill this with 0 if your items can be
    /// multi-drawn, or with a unique value if they can't.
    pub vertex_slab: SlabId,

    /// The ID of the slab of GPU memory that contains index data, if present.
    ///
    /// For non-mesh items, you can safely fill this with `None`.
    pub index_slab: Option<SlabId>,

    /// Index of the slab that the lightmap resides in, if a lightmap is
    /// present.
    pub lightmap_slab: Option<NonMaxU32>,
}

impl PhaseItemBatchSetKey for Opaque3dBatchSetKey {
    fn indexed(&self) -> bool {
        self.index_slab.is_some()
    }
}

/// Data that must be identical in order to *batch* phase items together.
///
/// Note that a *batch set* (if multi-draw is in use) contains multiple batches.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opaque3dBinKey {
    /// The asset that this phase item is associated with.
    ///
    /// Normally, this is the ID of the mesh, but for non-mesh items it might be
    /// the ID of another type of asset.
    pub asset_id: UntypedAssetId,
}

impl PhaseItem for Opaque3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
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
        self.extra_index.clone()
    }

    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for Opaque3d {
    type BatchSetKey = Opaque3dBatchSetKey;
    type BinKey = Opaque3dBinKey;

    #[inline]
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Opaque3d {
            batch_set_key,
            bin_key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}

pub struct AlphaMask3d {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: OpaqueNoLightmap3dBatchSetKey,
    /// The key, which determines which can be batched.
    pub bin_key: OpaqueNoLightmap3dBinKey,
    pub representative_entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for AlphaMask3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
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
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for AlphaMask3d {
    type BinKey = OpaqueNoLightmap3dBinKey;
    type BatchSetKey = OpaqueNoLightmap3dBatchSetKey;

    #[inline]
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            bin_key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}

pub struct Transmissive3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: (Entity, MainEntity),
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    /// Whether the mesh in question is indexed (uses an index buffer in
    /// addition to its vertex buffer).
    pub indexed: bool,
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
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
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
        self.extra_index.clone()
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

    #[inline]
    fn indexed(&self) -> bool {
        self.indexed
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
    pub entity: (Entity, MainEntity),
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    /// Whether the mesh in question is indexed (uses an index buffer in
    /// addition to its vertex buffer).
    pub indexed: bool,
}

impl PhaseItem for Transparent3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.entity.1
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
        self.extra_index.clone()
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

    #[inline]
    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for Transparent3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn prepare_core_3d_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (
            Entity,
            &ExtractedCamera,
            Option<&DepthPrepass>,
            &Camera3d,
            &Msaa,
        ),
        (
            With<BinnedRenderPhase<Opaque3d>>,
            With<BinnedRenderPhase<AlphaMask3d>>,
            With<SortedRenderPhase<Transmissive3d>>,
            With<SortedRenderPhase<Transparent3d>>,
        ),
    >,
) {
    let mut render_target_usage = <HashMap<_, _>>::default();
    for (_, camera, depth_prepass, camera_3d, _msaa) in &views_3d {
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

    let mut textures = <HashMap<_, _>>::default();
    for (entity, camera, _, camera_3d, msaa) in &views_3d {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let cached_texture = textures
            .entry((camera.target.clone(), msaa))
            .or_insert_with(|| {
                let usage = *render_target_usage
                    .get(&camera.target.clone())
                    .expect("The depth texture usage should already exist for this target");

                let descriptor = TextureDescriptor {
                    label: Some("view_depth_texture"),
                    // The size of the depth texture
                    size: physical_target_size.to_extents(),
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
            &SortedRenderPhase<Transmissive3d>,
        ),
        (
            With<BinnedRenderPhase<Opaque3d>>,
            With<BinnedRenderPhase<AlphaMask3d>>,
            With<SortedRenderPhase<Transparent3d>>,
        ),
    >,
) {
    let mut textures = <HashMap<_, _>>::default();
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

                let format = if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                };

                let descriptor = TextureDescriptor {
                    label: Some("view_transmission_texture"),
                    // The size of the transmission texture
                    size: physical_target_size.to_extents(),
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

/// Sets the `TEXTURE_BINDING` flag on the depth texture if necessary for
/// occlusion culling.
///
/// We need that flag to be set in order to read from the texture.
fn configure_occlusion_culling_view_targets(
    mut view_targets: Query<
        &mut Camera3d,
        (
            With<OcclusionCulling>,
            Without<NoIndirectDrawing>,
            With<DepthPrepass>,
        ),
    >,
) {
    for mut camera_3d in &mut view_targets {
        let mut depth_texture_usages = TextureUsages::from(camera_3d.depth_texture_usages);
        depth_texture_usages |= TextureUsages::TEXTURE_BINDING;
        camera_3d.depth_texture_usages = depth_texture_usages.into();
    }
}

// Disable MSAA and warn if using deferred rendering
pub fn check_msaa(mut deferred_views: Query<&mut Msaa, (With<Camera>, With<DeferredPrepass>)>) {
    for mut msaa in deferred_views.iter_mut() {
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
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    views_3d: Query<
        (
            Entity,
            &ExtractedCamera,
            &Msaa,
            Has<DepthPrepass>,
            Has<NormalPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
            Has<DepthPrepassDoubleBuffer>,
            Has<DeferredPrepassDoubleBuffer>,
        ),
        (
            Or<(
                With<BinnedRenderPhase<Opaque3dPrepass>>,
                With<BinnedRenderPhase<AlphaMask3dPrepass>>,
                With<BinnedRenderPhase<Opaque3dDeferred>>,
                With<BinnedRenderPhase<AlphaMask3dDeferred>>,
            )>,
        ),
    >,
) {
    let mut depth_textures1 = <HashMap<_, _>>::default();
    let mut depth_textures2 = <HashMap<_, _>>::default();
    let mut normal_textures = <HashMap<_, _>>::default();
    let mut deferred_textures1: HashMap<_, _> = <HashMap<_, _>>::default();
    let mut deferred_textures2: HashMap<_, _> = <HashMap<_, _>>::default();
    let mut deferred_lighting_id_textures = <HashMap<_, _>>::default();
    let mut motion_vectors_textures = <HashMap<_, _>>::default();
    for (
        entity,
        camera,
        msaa,
        depth_prepass,
        normal_prepass,
        motion_vector_prepass,
        deferred_prepass,
        depth_prepass_double_buffer,
        deferred_prepass_double_buffer,
    ) in &views_3d
    {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let size = physical_target_size.to_extents();

        let cached_depth_texture1 = depth_prepass.then(|| {
            depth_textures1
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    let descriptor = TextureDescriptor {
                        label: Some("prepass_depth_texture_1"),
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

        let cached_depth_texture2 = depth_prepass_double_buffer.then(|| {
            depth_textures2
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    let descriptor = TextureDescriptor {
                        label: Some("prepass_depth_texture_2"),
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

        let cached_deferred_texture1 = deferred_prepass.then(|| {
            deferred_textures1
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("prepass_deferred_texture_1"),
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

        let cached_deferred_texture2 = deferred_prepass_double_buffer.then(|| {
            deferred_textures2
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("prepass_deferred_texture_2"),
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
            depth: package_double_buffered_texture(
                cached_depth_texture1,
                cached_depth_texture2,
                frame_count.0,
            ),
            normal: cached_normals_texture
                .map(|t| ColorAttachment::new(t, None, None, Some(LinearRgba::BLACK))),
            // Red and Green channels are X and Y components of the motion vectors
            // Blue channel doesn't matter, but set to 0.0 for possible faster clear
            // https://gpuopen.com/performance/#clears
            motion_vectors: cached_motion_vectors_texture
                .map(|t| ColorAttachment::new(t, None, None, Some(LinearRgba::BLACK))),
            deferred: package_double_buffered_texture(
                cached_deferred_texture1,
                cached_deferred_texture2,
                frame_count.0,
            ),
            deferred_lighting_pass_id: cached_deferred_lighting_pass_id_texture
                .map(|t| ColorAttachment::new(t, None, None, Some(LinearRgba::BLACK))),
            size,
        });
    }
}

fn package_double_buffered_texture(
    texture1: Option<CachedTexture>,
    texture2: Option<CachedTexture>,
    frame_count: u32,
) -> Option<ColorAttachment> {
    match (texture1, texture2) {
        (Some(t1), None) => Some(ColorAttachment::new(
            t1,
            None,
            None,
            Some(LinearRgba::BLACK),
        )),
        (Some(t1), Some(t2)) if frame_count.is_multiple_of(2) => Some(ColorAttachment::new(
            t1,
            None,
            Some(t2),
            Some(LinearRgba::BLACK),
        )),
        (Some(t1), Some(t2)) => Some(ColorAttachment::new(
            t2,
            None,
            Some(t1),
            Some(LinearRgba::BLACK),
        )),
        _ => None,
    }
}
