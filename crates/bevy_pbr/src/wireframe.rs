use crate::{
    DrawMesh, MeshPipeline, MeshPipelineKey, RenderMeshInstanceFlags, RenderMeshInstances,
    SetMeshBindGroup, SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup, ViewKeyCache,
    ViewSpecializationTicks,
};
use bevy_app::{App, Plugin, PostUpdate, Startup, Update};
use bevy_asset::{
    embedded_asset, load_embedded_asset, prelude::AssetChanged, AsAssetId, Asset, AssetApp,
    AssetEventSystems, AssetId, AssetServer, Assets, Handle, UntypedAssetId,
};
use bevy_camera::{visibility::ViewVisibility, Camera, Camera3d};
use bevy_color::{Color, ColorToComponents};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Tick,
    prelude::*,
    query::QueryItem,
    system::{lifetimeless::SRes, SystemChangeTick, SystemParamItem},
};
use bevy_mesh::{Mesh3d, MeshVertexBufferLayoutRef};
use bevy_platform::{
    collections::{HashMap, HashSet},
    hash::FixedHasher,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    camera::{extract_cameras, ExtractedCamera},
    diagnostic::RecordDiagnostics,
    extract_resource::ExtractResource,
    mesh::{
        allocator::{MeshAllocator, SlabId},
        RenderMesh,
    },
    prelude::*,
    render_asset::{
        prepare_assets, PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets,
    },
    render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner},
    render_phase::{
        AddRenderCommand, BinnedPhaseItem, BinnedRenderPhasePlugin, BinnedRenderPhaseType,
        CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
        PhaseItemBatchSetKey, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
        SetItemPipeline, TrackedRenderPass, ViewBinnedRenderPhases,
    },
    render_resource::*,
    renderer::RenderContext,
    sync_world::{MainEntity, MainEntityHashMap},
    view::{
        ExtractedView, NoIndirectDrawing, RenderVisibilityRanges, RenderVisibleEntities,
        RetainedViewEntity, ViewDepthTexture, ViewTarget,
    },
    Extract, Render, RenderApp, RenderDebugFlags, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use core::{hash::Hash, ops::Range};
use tracing::error;

/// A [`Plugin`] that draws wireframes.
///
/// Wireframes currently do not work when using webgl or webgpu.
/// Supported rendering backends:
/// - DX12
/// - Vulkan
/// - Metal
///
/// This is a native only feature.
#[derive(Debug, Default)]
pub struct WireframePlugin {
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
}

impl WireframePlugin {
    /// Creates a new [`WireframePlugin`] with the given debug flags.
    pub fn new(debug_flags: RenderDebugFlags) -> Self {
        Self { debug_flags }
    }
}

impl Plugin for WireframePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "render/wireframe.wgsl");

        app.add_plugins((
            BinnedRenderPhasePlugin::<Wireframe3d, MeshPipeline>::new(self.debug_flags),
            RenderAssetPlugin::<RenderWireframeMaterial>::default(),
        ))
        .init_asset::<WireframeMaterial>()
        .init_resource::<SpecializedMeshPipelines<Wireframe3dPipeline>>()
        .init_resource::<WireframeConfig>()
        .init_resource::<WireframeEntitiesNeedingSpecialization>()
        .add_systems(Startup, setup_global_wireframe_material)
        .add_systems(
            Update,
            (
                global_color_changed.run_if(resource_changed::<WireframeConfig>),
                wireframe_color_changed,
                // Run `apply_global_wireframe_material` after `apply_wireframe_material` so that the global
                // wireframe setting is applied to a mesh on the same frame its wireframe marker component is removed.
                (apply_wireframe_material, apply_global_wireframe_material).chain(),
            ),
        )
        .add_systems(
            PostUpdate,
            check_wireframe_entities_needing_specialization
                .after(AssetEventSystems)
                .run_if(resource_exists::<WireframeConfig>),
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<WireframeEntitySpecializationTicks>()
            .init_resource::<SpecializedWireframePipelineCache>()
            .init_resource::<DrawFunctions<Wireframe3d>>()
            .add_render_command::<Wireframe3d, DrawWireframe3d>()
            .init_resource::<RenderWireframeInstances>()
            .init_resource::<SpecializedMeshPipelines<Wireframe3dPipeline>>()
            .add_render_graph_node::<ViewNodeRunner<Wireframe3dNode>>(Core3d, Node3d::Wireframe)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    Node3d::Wireframe,
                    Node3d::PostProcessing,
                ),
            )
            .add_systems(RenderStartup, init_wireframe_3d_pipeline)
            .add_systems(
                ExtractSchedule,
                (
                    extract_wireframe_3d_camera,
                    extract_wireframe_entities_needing_specialization.after(extract_cameras),
                    extract_wireframe_materials,
                ),
            )
            .add_systems(
                Render,
                (
                    specialize_wireframes
                        .in_set(RenderSystems::PrepareMeshes)
                        .after(prepare_assets::<RenderWireframeMaterial>)
                        .after(prepare_assets::<RenderMesh>),
                    queue_wireframes
                        .in_set(RenderSystems::QueueMeshes)
                        .after(prepare_assets::<RenderWireframeMaterial>),
                ),
            );
    }
}

/// Enables wireframe rendering for any entity it is attached to.
/// It will ignore the [`WireframeConfig`] global setting.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct Wireframe;

pub struct Wireframe3d {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: Wireframe3dBatchSetKey,
    /// The key, which determines which can be batched.
    pub bin_key: Wireframe3dBinKey,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub representative_entity: (Entity, MainEntity),
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for Wireframe3d {
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
    }

    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

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

impl CachedRenderPipelinePhaseItem for Wireframe3d {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}

impl BinnedPhaseItem for Wireframe3d {
    type BinKey = Wireframe3dBinKey;
    type BatchSetKey = Wireframe3dBatchSetKey;

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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wireframe3dBatchSetKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,

    /// The wireframe material asset ID.
    pub asset_id: UntypedAssetId,

    /// The function used to draw.
    pub draw_function: DrawFunctionId,
    /// The ID of the slab of GPU memory that contains vertex data.
    ///
    /// For non-mesh items, you can fill this with 0 if your items can be
    /// multi-drawn, or with a unique value if they can't.
    pub vertex_slab: SlabId,

    /// The ID of the slab of GPU memory that contains index data, if present.
    ///
    /// For non-mesh items, you can safely fill this with `None`.
    pub index_slab: Option<SlabId>,
}

impl PhaseItemBatchSetKey for Wireframe3dBatchSetKey {
    fn indexed(&self) -> bool {
        self.index_slab.is_some()
    }
}

/// Data that must be identical in order to *batch* phase items together.
///
/// Note that a *batch set* (if multi-draw is in use) contains multiple batches.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wireframe3dBinKey {
    /// The wireframe mesh asset ID.
    pub asset_id: UntypedAssetId,
}

pub struct SetWireframe3dPushConstants;

impl<P: PhaseItem> RenderCommand<P> for SetWireframe3dPushConstants {
    type Param = (
        SRes<RenderWireframeInstances>,
        SRes<RenderAssets<RenderWireframeMaterial>>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (wireframe_instances, wireframe_assets): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(wireframe_material) = wireframe_instances.get(&item.main_entity()) else {
            return RenderCommandResult::Failure("No wireframe material found for entity");
        };
        let Some(wireframe_material) = wireframe_assets.get(*wireframe_material) else {
            return RenderCommandResult::Failure("No wireframe material found for entity");
        };

        pass.set_push_constants(
            ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&wireframe_material.color),
        );
        RenderCommandResult::Success
    }
}

pub type DrawWireframe3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetWireframe3dPushConstants,
    DrawMesh,
);

#[derive(Resource, Clone)]
pub struct Wireframe3dPipeline {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
}

pub fn init_wireframe_3d_pipeline(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(Wireframe3dPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        shader: load_embedded_asset!(asset_server.as_ref(), "render/wireframe.wgsl"),
    });
}

impl SpecializedMeshPipeline for Wireframe3dPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.label = Some("wireframe_3d_pipeline".into());
        descriptor.push_constant_ranges.push(PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..16,
        });
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader = self.shader.clone();
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        descriptor.depth_stencil.as_mut().unwrap().bias.slope_scale = 1.0;
        Ok(descriptor)
    }
}

#[derive(Default)]
struct Wireframe3dNode;
impl ViewNode for Wireframe3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view, target, depth): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(wireframe_phase) = world.get_resource::<ViewBinnedRenderPhases<Wireframe3d>>()
        else {
            return Ok(());
        };

        let Some(wireframe_phase) = wireframe_phase.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("wireframe_3d"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let pass_span = diagnostics.pass_span(&mut render_pass, "wireframe_3d");

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        if let Err(err) = wireframe_phase.render(&mut render_pass, world, graph.view_entity()) {
            error!("Error encountered while rendering the stencil phase {err:?}");
            return Err(NodeRunError::DrawError(err));
        }

        pass_span.end(&mut render_pass);

        Ok(())
    }
}

/// Sets the color of the [`Wireframe`] of the entity it is attached to.
///
/// If this component is present but there's no [`Wireframe`] component,
/// it will still affect the color of the wireframe when [`WireframeConfig::global`] is set to true.
///
/// This overrides the [`WireframeConfig::default_color`].
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct WireframeColor {
    pub color: Color,
}

#[derive(Component, Debug, Clone, Default)]
pub struct ExtractedWireframeColor {
    pub color: [f32; 4],
}

/// Disables wireframe rendering for any entity it is attached to.
/// It will ignore the [`WireframeConfig`] global setting.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct NoWireframe;

#[derive(Resource, Debug, Clone, Default, ExtractResource, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct WireframeConfig {
    /// Whether to show wireframes for all meshes.
    /// Can be overridden for individual meshes by adding a [`Wireframe`] or [`NoWireframe`] component.
    pub global: bool,
    /// If [`Self::global`] is set, any [`Entity`] that does not have a [`Wireframe`] component attached to it will have
    /// wireframes using this color. Otherwise, this will be the fallback color for any entity that has a [`Wireframe`],
    /// but no [`WireframeColor`].
    pub default_color: Color,
}

#[derive(Asset, Reflect, Clone, Debug, Default)]
#[reflect(Clone, Default)]
pub struct WireframeMaterial {
    pub color: Color,
}

pub struct RenderWireframeMaterial {
    pub color: [f32; 4],
}

#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct Mesh3dWireframe(pub Handle<WireframeMaterial>);

impl AsAssetId for Mesh3dWireframe {
    type Asset = WireframeMaterial;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.0.id()
    }
}

impl RenderAsset for RenderWireframeMaterial {
    type SourceAsset = WireframeMaterial;
    type Param = ();

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _asset_id: AssetId<Self::SourceAsset>,
        _param: &mut SystemParamItem<Self::Param>,
        _previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        Ok(RenderWireframeMaterial {
            color: source_asset.color.to_linear().to_f32_array(),
        })
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenderWireframeInstances(MainEntityHashMap<AssetId<WireframeMaterial>>);

#[derive(Clone, Resource, Deref, DerefMut, Debug, Default)]
pub struct WireframeEntitiesNeedingSpecialization {
    #[deref]
    pub entities: Vec<Entity>,
}

#[derive(Resource, Deref, DerefMut, Clone, Debug, Default)]
pub struct WireframeEntitySpecializationTicks {
    pub entities: MainEntityHashMap<Tick>,
}

/// Stores the [`SpecializedWireframeViewPipelineCache`] for each view.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct SpecializedWireframePipelineCache {
    // view entity -> view pipeline cache
    #[deref]
    map: HashMap<RetainedViewEntity, SpecializedWireframeViewPipelineCache>,
}

/// Stores the cached render pipeline ID for each entity in a single view, as
/// well as the last time it was changed.
#[derive(Deref, DerefMut, Default)]
pub struct SpecializedWireframeViewPipelineCache {
    // material entity -> (tick, pipeline_id)
    #[deref]
    map: MainEntityHashMap<(Tick, CachedRenderPipelineId)>,
}

#[derive(Resource)]
struct GlobalWireframeMaterial {
    // This handle will be reused when the global config is enabled
    handle: Handle<WireframeMaterial>,
}

pub fn extract_wireframe_materials(
    mut material_instances: ResMut<RenderWireframeInstances>,
    changed_meshes_query: Extract<
        Query<
            (Entity, &ViewVisibility, &Mesh3dWireframe),
            Or<(Changed<ViewVisibility>, Changed<Mesh3dWireframe>)>,
        >,
    >,
    mut removed_visibilities_query: Extract<RemovedComponents<ViewVisibility>>,
    mut removed_materials_query: Extract<RemovedComponents<Mesh3dWireframe>>,
) {
    for (entity, view_visibility, material) in &changed_meshes_query {
        if view_visibility.get() {
            material_instances.insert(entity.into(), material.id());
        } else {
            material_instances.remove(&MainEntity::from(entity));
        }
    }

    for entity in removed_visibilities_query
        .read()
        .chain(removed_materials_query.read())
    {
        // Only queue a mesh for removal if we didn't pick it up above.
        // It's possible that a necessary component was removed and re-added in
        // the same frame.
        if !changed_meshes_query.contains(entity) {
            material_instances.remove(&MainEntity::from(entity));
        }
    }
}

fn setup_global_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    config: Res<WireframeConfig>,
) {
    // Create the handle used for the global material
    commands.insert_resource(GlobalWireframeMaterial {
        handle: materials.add(WireframeMaterial {
            color: config.default_color,
        }),
    });
}

/// Updates the wireframe material of all entities without a [`WireframeColor`] or without a [`Wireframe`] component
fn global_color_changed(
    config: Res<WireframeConfig>,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    global_material: Res<GlobalWireframeMaterial>,
) {
    if let Some(global_material) = materials.get_mut(&global_material.handle) {
        global_material.color = config.default_color;
    }
}

/// Updates the wireframe material when the color in [`WireframeColor`] changes
fn wireframe_color_changed(
    mut materials: ResMut<Assets<WireframeMaterial>>,
    mut colors_changed: Query<
        (&mut Mesh3dWireframe, &WireframeColor),
        (With<Wireframe>, Changed<WireframeColor>),
    >,
) {
    for (mut handle, wireframe_color) in &mut colors_changed {
        handle.0 = materials.add(WireframeMaterial {
            color: wireframe_color.color,
        });
    }
}

/// Applies or remove the wireframe material to any mesh with a [`Wireframe`] component, and removes it
/// for any mesh with a [`NoWireframe`] component.
fn apply_wireframe_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<WireframeMaterial>>,
    wireframes: Query<
        (Entity, Option<&WireframeColor>),
        (With<Wireframe>, Without<Mesh3dWireframe>),
    >,
    no_wireframes: Query<Entity, (With<NoWireframe>, With<Mesh3dWireframe>)>,
    mut removed_wireframes: RemovedComponents<Wireframe>,
    global_material: Res<GlobalWireframeMaterial>,
) {
    for e in removed_wireframes.read().chain(no_wireframes.iter()) {
        if let Ok(mut commands) = commands.get_entity(e) {
            commands.remove::<Mesh3dWireframe>();
        }
    }

    let mut material_to_spawn = vec![];
    for (e, maybe_color) in &wireframes {
        let material = get_wireframe_material(maybe_color, &mut materials, &global_material);
        material_to_spawn.push((e, Mesh3dWireframe(material)));
    }
    commands.try_insert_batch(material_to_spawn);
}

type WireframeFilter = (With<Mesh3d>, Without<Wireframe>, Without<NoWireframe>);

/// Applies or removes a wireframe material on any mesh without a [`Wireframe`] or [`NoWireframe`] component.
fn apply_global_wireframe_material(
    mut commands: Commands,
    config: Res<WireframeConfig>,
    meshes_without_material: Query<
        (Entity, Option<&WireframeColor>),
        (WireframeFilter, Without<Mesh3dWireframe>),
    >,
    meshes_with_global_material: Query<Entity, (WireframeFilter, With<Mesh3dWireframe>)>,
    global_material: Res<GlobalWireframeMaterial>,
    mut materials: ResMut<Assets<WireframeMaterial>>,
) {
    if config.global {
        let mut material_to_spawn = vec![];
        for (e, maybe_color) in &meshes_without_material {
            let material = get_wireframe_material(maybe_color, &mut materials, &global_material);
            // We only add the material handle but not the Wireframe component
            // This makes it easy to detect which mesh is using the global material and which ones are user specified
            material_to_spawn.push((e, Mesh3dWireframe(material)));
        }
        commands.try_insert_batch(material_to_spawn);
    } else {
        for e in &meshes_with_global_material {
            commands.entity(e).remove::<Mesh3dWireframe>();
        }
    }
}

/// Gets a handle to a wireframe material with a fallback on the default material
fn get_wireframe_material(
    maybe_color: Option<&WireframeColor>,
    wireframe_materials: &mut Assets<WireframeMaterial>,
    global_material: &GlobalWireframeMaterial,
) -> Handle<WireframeMaterial> {
    if let Some(wireframe_color) = maybe_color {
        wireframe_materials.add(WireframeMaterial {
            color: wireframe_color.color,
        })
    } else {
        // If there's no color specified we can use the global material since it's already set to use the default_color
        global_material.handle.clone()
    }
}

fn extract_wireframe_3d_camera(
    mut wireframe_3d_phases: ResMut<ViewBinnedRenderPhases<Wireframe3d>>,
    cameras: Extract<Query<(Entity, &Camera, Has<NoIndirectDrawing>), With<Camera3d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
) {
    live_entities.clear();
    for (main_entity, camera, no_indirect_drawing) in &cameras {
        if !camera.is_active {
            continue;
        }
        let gpu_preprocessing_mode = gpu_preprocessing_support.min(if !no_indirect_drawing {
            GpuPreprocessingMode::Culling
        } else {
            GpuPreprocessingMode::PreprocessingOnly
        });

        let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);
        wireframe_3d_phases.prepare_for_new_frame(retained_view_entity, gpu_preprocessing_mode);
        live_entities.insert(retained_view_entity);
    }

    // Clear out all dead views.
    wireframe_3d_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

pub fn extract_wireframe_entities_needing_specialization(
    entities_needing_specialization: Extract<Res<WireframeEntitiesNeedingSpecialization>>,
    mut entity_specialization_ticks: ResMut<WireframeEntitySpecializationTicks>,
    views: Query<&ExtractedView>,
    mut specialized_wireframe_pipeline_cache: ResMut<SpecializedWireframePipelineCache>,
    mut removed_meshes_query: Extract<RemovedComponents<Mesh3d>>,
    ticks: SystemChangeTick,
) {
    for entity in entities_needing_specialization.iter() {
        // Update the entity's specialization tick with this run's tick
        entity_specialization_ticks.insert((*entity).into(), ticks.this_run());
    }

    for entity in removed_meshes_query.read() {
        for view in &views {
            if let Some(specialized_wireframe_pipeline_cache) =
                specialized_wireframe_pipeline_cache.get_mut(&view.retained_view_entity)
            {
                specialized_wireframe_pipeline_cache.remove(&MainEntity::from(entity));
            }
        }
    }
}

pub fn check_wireframe_entities_needing_specialization(
    needs_specialization: Query<
        Entity,
        Or<(
            Changed<Mesh3d>,
            AssetChanged<Mesh3d>,
            Changed<Mesh3dWireframe>,
            AssetChanged<Mesh3dWireframe>,
        )>,
    >,
    mut entities_needing_specialization: ResMut<WireframeEntitiesNeedingSpecialization>,
) {
    entities_needing_specialization.clear();
    for entity in &needs_specialization {
        entities_needing_specialization.push(entity);
    }
}

pub fn specialize_wireframes(
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_wireframe_instances: Res<RenderWireframeInstances>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    wireframe_phases: Res<ViewBinnedRenderPhases<Wireframe3d>>,
    views: Query<(&ExtractedView, &RenderVisibleEntities)>,
    view_key_cache: Res<ViewKeyCache>,
    entity_specialization_ticks: Res<WireframeEntitySpecializationTicks>,
    view_specialization_ticks: Res<ViewSpecializationTicks>,
    mut specialized_material_pipeline_cache: ResMut<SpecializedWireframePipelineCache>,
    mut pipelines: ResMut<SpecializedMeshPipelines<Wireframe3dPipeline>>,
    pipeline: Res<Wireframe3dPipeline>,
    pipeline_cache: Res<PipelineCache>,
    ticks: SystemChangeTick,
) {
    // Record the retained IDs of all views so that we can expire old
    // pipeline IDs.
    let mut all_views: HashSet<RetainedViewEntity, FixedHasher> = HashSet::default();

    for (view, visible_entities) in &views {
        all_views.insert(view.retained_view_entity);

        if !wireframe_phases.contains_key(&view.retained_view_entity) {
            continue;
        }

        let Some(view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        let view_tick = view_specialization_ticks
            .get(&view.retained_view_entity)
            .unwrap();
        let view_specialized_material_pipeline_cache = specialized_material_pipeline_cache
            .entry(view.retained_view_entity)
            .or_default();

        for (_, visible_entity) in visible_entities.iter::<Mesh3d>() {
            if !render_wireframe_instances.contains_key(visible_entity) {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let entity_tick = entity_specialization_ticks.get(visible_entity).unwrap();
            let last_specialized_tick = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .map(|(tick, _)| *tick);
            let needs_specialization = last_specialized_tick.is_none_or(|tick| {
                view_tick.is_newer_than(tick, ticks.this_run())
                    || entity_tick.is_newer_than(tick, ticks.this_run())
            });
            if !needs_specialization {
                continue;
            }
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let mut mesh_key = *view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

            if render_visibility_ranges.entity_has_crossfading_visibility_ranges(*visible_entity) {
                mesh_key |= MeshPipelineKey::VISIBILITY_RANGE_DITHER;
            }

            if view_key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
                // If the previous frame have skins or morph targets, note that.
                if mesh_instance
                    .flags
                    .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_SKIN)
                {
                    mesh_key |= MeshPipelineKey::HAS_PREVIOUS_SKIN;
                }
                if mesh_instance
                    .flags
                    .contains(RenderMeshInstanceFlags::HAS_PREVIOUS_MORPH)
                {
                    mesh_key |= MeshPipelineKey::HAS_PREVIOUS_MORPH;
                }
            }

            let pipeline_id =
                pipelines.specialize(&pipeline_cache, &pipeline, mesh_key, &mesh.layout);
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            view_specialized_material_pipeline_cache
                .insert(*visible_entity, (ticks.this_run(), pipeline_id));
        }
    }

    // Delete specialized pipelines belonging to views that have expired.
    specialized_material_pipeline_cache
        .retain(|retained_view_entity, _| all_views.contains(retained_view_entity));
}

fn queue_wireframes(
    custom_draw_functions: Res<DrawFunctions<Wireframe3d>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mesh_allocator: Res<MeshAllocator>,
    specialized_wireframe_pipeline_cache: Res<SpecializedWireframePipelineCache>,
    render_wireframe_instances: Res<RenderWireframeInstances>,
    mut wireframe_3d_phases: ResMut<ViewBinnedRenderPhases<Wireframe3d>>,
    mut views: Query<(&ExtractedView, &RenderVisibleEntities)>,
) {
    for (view, visible_entities) in &mut views {
        let Some(wireframe_phase) = wireframe_3d_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let draw_wireframe = custom_draw_functions.read().id::<DrawWireframe3d>();

        let Some(view_specialized_material_pipeline_cache) =
            specialized_wireframe_pipeline_cache.get(&view.retained_view_entity)
        else {
            continue;
        };

        for (render_entity, visible_entity) in visible_entities.iter::<Mesh3d>() {
            let Some(wireframe_instance) = render_wireframe_instances.get(visible_entity) else {
                continue;
            };
            let Some((current_change_tick, pipeline_id)) = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .map(|(current_change_tick, pipeline_id)| (*current_change_tick, *pipeline_id))
            else {
                continue;
            };

            // Skip the entity if it's cached in a bin and up to date.
            if wireframe_phase.validate_cached_entity(*visible_entity, current_change_tick) {
                continue;
            }
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let (vertex_slab, index_slab) = mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);
            let bin_key = Wireframe3dBinKey {
                asset_id: mesh_instance.mesh_asset_id.untyped(),
            };
            let batch_set_key = Wireframe3dBatchSetKey {
                pipeline: pipeline_id,
                asset_id: wireframe_instance.untyped(),
                draw_function: draw_wireframe,
                vertex_slab: vertex_slab.unwrap_or_default(),
                index_slab,
            };
            wireframe_phase.add(
                batch_set_key,
                bin_key,
                (*render_entity, *visible_entity),
                mesh_instance.current_uniform_index,
                BinnedRenderPhaseType::mesh(
                    mesh_instance.should_batch(),
                    &gpu_preprocessing_support,
                ),
                current_change_tick,
            );
        }
    }
}
