mod camera_2d;
mod main_opaque_pass_2d_node;
mod main_transparent_pass_2d_node;

pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct Core2d;

    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum Node2d {
        MsaaWriteback,
        StartMainPass,
        MainOpaquePass,
        MainTransparentPass,
        EndMainPass,
        Bloom,
        PostProcessing,
        Tonemapping,
        Fxaa,
        Smaa,
        Upscaling,
        ContrastAdaptiveSharpening,
        EndMainPassPostProcessing,
    }
}

use std::ops::Range;

use bevy_asset::UntypedAssetId;
use bevy_utils::HashMap;
pub use camera_2d::*;
pub use main_opaque_pass_2d_node::*;
pub use main_transparent_pass_2d_node::*;

use bevy_app::{App, Plugin};
use bevy_ecs::{entity::EntityHashSet, prelude::*};
use bevy_math::FloatOrd;
use bevy_render::{
    camera::{Camera, ExtractedCamera},
    extract_component::ExtractComponentPlugin,
    render_graph::{EmptyNode, RenderGraphApp, ViewNodeRunner},
    render_phase::{
        sort_phase_system, BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId,
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, SortedPhaseItem, ViewBinnedRenderPhases,
        ViewSortedRenderPhases,
    },
    render_resource::{
        BindGroupId, CachedRenderPipelineId, Extent3d, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages,
    },
    renderer::RenderDevice,
    texture::TextureCache,
    view::{Msaa, ViewDepthTexture},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};

use crate::{tonemapping::TonemappingNode, upscaling::UpscalingNode};

use self::graph::{Core2d, Node2d};

pub const CORE_2D_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub struct Core2dPlugin;

impl Plugin for Core2dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera2d>()
            .add_plugins(ExtractComponentPlugin::<Camera2d>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<DrawFunctions<Opaque2d>>()
            .init_resource::<DrawFunctions<AlphaMask2d>>()
            .init_resource::<DrawFunctions<Transparent2d>>()
            .init_resource::<ViewSortedRenderPhases<Transparent2d>>()
            .init_resource::<ViewBinnedRenderPhases<Opaque2d>>()
            .init_resource::<ViewBinnedRenderPhases<AlphaMask2d>>()
            .add_systems(ExtractSchedule, extract_core_2d_camera_phases)
            .add_systems(
                Render,
                (
                    sort_phase_system::<Transparent2d>.in_set(RenderSet::PhaseSort),
                    prepare_core_2d_depth_textures.in_set(RenderSet::PrepareResources),
                ),
            );

        render_app
            .add_render_sub_graph(Core2d)
            .add_render_graph_node::<EmptyNode>(Core2d, Node2d::StartMainPass)
            .add_render_graph_node::<ViewNodeRunner<MainOpaquePass2dNode>>(
                Core2d,
                Node2d::MainOpaquePass,
            )
            .add_render_graph_node::<ViewNodeRunner<MainTransparentPass2dNode>>(
                Core2d,
                Node2d::MainTransparentPass,
            )
            .add_render_graph_node::<EmptyNode>(Core2d, Node2d::EndMainPass)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(Core2d, Node2d::Tonemapping)
            .add_render_graph_node::<EmptyNode>(Core2d, Node2d::EndMainPassPostProcessing)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(Core2d, Node2d::Upscaling)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::StartMainPass,
                    Node2d::MainOpaquePass,
                    Node2d::MainTransparentPass,
                    Node2d::EndMainPass,
                    Node2d::Tonemapping,
                    Node2d::EndMainPassPostProcessing,
                    Node2d::Upscaling,
                ),
            );
    }
}

/// Opaque 2D [`BinnedPhaseItem`]s.
pub struct Opaque2d {
    /// The key, which determines which can be batched.
    pub key: Opaque2dBinKey,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub representative_entity: Entity,
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

/// Data that must be identical in order to batch phase items together.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Opaque2dBinKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,
    /// The function used to draw.
    pub draw_function: DrawFunctionId,
    /// The asset that this phase item is associated with.
    ///
    /// Normally, this is the ID of the mesh, but for non-mesh items it might be
    /// the ID of another type of asset.
    pub asset_id: UntypedAssetId,
    /// The ID of a bind group specific to the material.
    pub material_bind_group_id: Option<BindGroupId>,
}

impl PhaseItem for Opaque2d {
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

impl BinnedPhaseItem for Opaque2d {
    type BinKey = Opaque2dBinKey;

    fn new(
        key: Self::BinKey,
        representative_entity: Entity,
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Opaque2d {
            key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for Opaque2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.key.pipeline
    }
}

/// Alpha mask 2D [`BinnedPhaseItem`]s.
pub struct AlphaMask2d {
    /// The key, which determines which can be batched.
    pub key: AlphaMask2dBinKey,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub representative_entity: Entity,
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

/// Data that must be identical in order to batch phase items together.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlphaMask2dBinKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,
    /// The function used to draw.
    pub draw_function: DrawFunctionId,
    /// The asset that this phase item is associated with.
    ///
    /// Normally, this is the ID of the mesh, but for non-mesh items it might be
    /// the ID of another type of asset.
    pub asset_id: UntypedAssetId,
    /// The ID of a bind group specific to the material.
    pub material_bind_group_id: Option<BindGroupId>,
}

impl PhaseItem for AlphaMask2d {
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

impl BinnedPhaseItem for AlphaMask2d {
    type BinKey = AlphaMask2dBinKey;

    fn new(
        key: Self::BinKey,
        representative_entity: Entity,
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        AlphaMask2d {
            key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.key.pipeline
    }
}

/// Transparent 2D [`SortedPhaseItem`]s.
pub struct Transparent2d {
    pub sort_key: FloatOrd,
    pub entity: Entity,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for Transparent2d {
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

impl SortedPhaseItem for Transparent2d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // radsort is a stable radix sort that performed better than `slice::sort_by_key` or `slice::sort_unstable_by_key`.
        radsort::sort_by_key(items, |item| item.sort_key().0);
    }
}

impl CachedRenderPipelinePhaseItem for Transparent2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_core_2d_camera_phases(
    mut commands: Commands,
    mut transparent_2d_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut opaque_2d_phases: ResMut<ViewBinnedRenderPhases<Opaque2d>>,
    mut alpha_mask_2d_phases: ResMut<ViewBinnedRenderPhases<AlphaMask2d>>,
    cameras_2d: Extract<Query<(Entity, &Camera), With<Camera2d>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();

    for (entity, camera) in &cameras_2d {
        if !camera.is_active {
            continue;
        }

        commands.get_or_spawn(entity);
        transparent_2d_phases.insert_or_clear(entity);
        opaque_2d_phases.insert_or_clear(entity);
        alpha_mask_2d_phases.insert_or_clear(entity);

        live_entities.insert(entity);
    }

    // Clear out all dead views.
    transparent_2d_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
    opaque_2d_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
    alpha_mask_2d_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

pub fn prepare_core_2d_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    transparent_2d_phases: Res<ViewSortedRenderPhases<Transparent2d>>,
    opaque_2d_phases: Res<ViewBinnedRenderPhases<Opaque2d>>,
    views_2d: Query<(Entity, &ExtractedCamera, &Msaa), (With<Camera2d>,)>,
) {
    let mut textures = HashMap::default();
    for (view, camera, msaa) in &views_2d {
        if !opaque_2d_phases.contains_key(&view) || !transparent_2d_phases.contains_key(&view) {
            continue;
        };

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

                let descriptor = TextureDescriptor {
                    label: Some("view_depth_texture"),
                    size,
                    mip_level_count: 1,
                    sample_count: msaa.samples(),
                    dimension: TextureDimension::D2,
                    format: CORE_2D_DEPTH_FORMAT,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                };

                texture_cache.get(&render_device, descriptor)
            })
            .clone();

        commands
            .entity(view)
            .insert(ViewDepthTexture::new(cached_texture, Some(0.0)));
    }
}
