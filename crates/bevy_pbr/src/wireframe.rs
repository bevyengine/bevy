use crate::material_bind_groups::MaterialBindGroupAllocator;
use crate::{DrawMesh, Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshMaterial3d, MeshPipeline, MeshPipelineKey, PreparedMaterial, RenderMaterialInstances, RenderMeshInstances, SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup};
use bevy_app::{Plugin, Startup, Update};
use bevy_asset::{
    load_internal_asset, weak_handle, Asset, AssetApp, Assets, Handle, UntypedAssetId,
};
use bevy_color::{Color, ColorToComponents, LinearRgba};
use bevy_core_pipeline::core_3d::{Camera3d, Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey};
use bevy_ecs::entity::hash_map::EntityHashMap;
use bevy_ecs::entity::hash_set::EntityHashSet;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_ecs::system::lifetimeless::SRes;
use bevy_ecs::system::SystemParamItem;
use bevy_math::{FloatOrd, Vec4};
use bevy_platform_support::collections::{HashMap, HashSet};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::camera::ExtractedCamera;
use bevy_render::extract_component::UniformComponentPlugin;
use bevy_render::mesh::allocator::SlabId;
use bevy_render::mesh::{MeshVertexBufferLayout, RenderMesh};
use bevy_render::render_asset::RenderAssets;
use bevy_render::render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode};
use bevy_render::render_phase::{BinnedPhaseItem, BinnedRenderPhasePlugin, DrawFunctionId, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewBinnedRenderPhases, ViewSortedRenderPhases};
use bevy_render::render_resource::binding_types::{
    sampler, storage_buffer, storage_buffer_read_only, texture_2d, uniform_buffer,
};
use bevy_render::renderer::{RenderContext, RenderDevice};
use bevy_render::sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet};
use bevy_render::view::{ExtractedView, NoIndirectDrawing, RenderVisibleEntities, RetainedViewEntity, ViewDepthTexture, ViewTarget};
use bevy_render::{
    extract_resource::ExtractResource,
    mesh::{Mesh3d, MeshVertexBufferLayoutRef},
    prelude::*,
    render_resource::*,
    Extract, RenderApp,
};
use nonmax::NonMaxU32;
use std::ops::Range;
use tracing::error;
use bevy_render::batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport};

pub const WIREFRAME_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("2646a633-f8e3-4380-87ae-b44d881abbce");

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
pub struct WireframePlugin;
impl Plugin for WireframePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            WIREFRAME_SHADER_HANDLE,
            "render/wireframe.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((BinnedRenderPhasePlugin::<Wireframe3d, MeshPipeline>::default(),))
            .init_resource::<SpecializedMeshPipelines<Wireframe3dPipeline>>()
            .register_type::<Wireframe>()
            .register_type::<NoWireframe>()
            .register_type::<WireframeConfig>()
            .register_type::<WireframeColor>()
            .init_resource::<WireframeConfig>();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
    }
}

/// Enables wireframe rendering for any entity it is attached to.
/// It will ignore the [`WireframeConfig`] global setting.
///
/// This requires the [`WireframePlugin`] to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct Wireframe;

struct Wireframe3d {
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
        todo!()
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

struct Wireframe3dBatchSetKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,

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

/// Data that must be identical in order to *batch* phase items together.
///
/// Note that a *batch set* (if multi-draw is in use) contains multiple batches.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wireframe3dBinKey {
    pub color: Entity,
}

pub struct SetWireframe3dPushConstants<const N: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetWireframe3dPushConstants<I> {
    type Param = (SRes<ExtractedWireframeConfig>);
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (wireframe_config): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_push_constants(
            ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&wireframe_config.color_for_entity(item.main_entity())),
        );
        RenderCommandResult::Success
    }
}

pub type DrawWireframe3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetWireframe3dPushConstants<2>,
    DrawMesh,
);

pub struct Wireframe3dPipelineKey {
    mesh_key: MeshPipelineKey,
    color: [f32; 4],
}

#[derive(Resource, Clone)]
pub struct Wireframe3dPipeline {
    mesh_pipeline: MeshPipeline,
    shader: Handle<Shader>,
    layout: BindGroupLayout,
}

impl FromWorld for Wireframe3dPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "wireframe_material_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (storage_buffer_read_only::<Vec4>(false),),
            ),
        );

        Wireframe3dPipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            layout,
            shader: WIREFRAME_SHADER_HANDLE,
        }
    }
}

impl SpecializedMeshPipeline for Wireframe3dPipeline {
    type Key = Wireframe3dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.mesh_key, layout)?;
        descriptor.layout.push(self.layout.clone());
        descriptor.push_constant_ranges.push(PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..4,
        });
        descriptor.fragment.unwrap().shader = self.shader.clone();
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        descriptor.depth_stencil.as_mut().unwrap().bias.slope_scale = 1.0;
        Ok(())
    }
}

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct Wireframe3dLabel;

#[derive(Default)]
struct Wireframe3dNode;
impl ViewNode for Wireframe3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view, target): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(wireframe_phase) = world.get_resource::<ViewBinnedRenderPhases<Wireframe3d>>()
        else {
            return Ok(());
        };

        let Some(wireframe_phase) = wireframe_phase.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("wireframe_3d_pass"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        if let Err(err) = wireframe_phase.render(&mut render_pass, world, graph.view_entity()) {
            error!("Error encountered while rendering the stencil phase {err:?}");
            return Err(NodeRunError::DrawError(err));
        }

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

pub struct ExtractedWireframeConfig {
    pub global: bool,
    pub default_color: [f32; 4],
    pub color_entities: HashMap<[f32; 4], Entity>,
    pub color_instances: MainEntityHashMap<Entity>,
    pub no_wireframe_instances: MainEntityHashSet,
}

impl ExtractedWireframeConfig {
    pub fn color_for_entity(&self, main_entity: MainEntity) -> Option<[f32; 4]> {
        if let Some(color) = self.color_instances.get(&main_entity) {
            return Some(*color);
        }
        if self.no_wireframe_instances.contains(&main_entity) {
            return None;
        }
        Some(self.default_color)
    }
}

/// Updates the wireframe material when the color in [`WireframeColor`] changes
fn extract_wireframe_colors(
    mut commands: Commands,
    config: Extract<Res<WireframeConfig>>,
    mut extracted_config: ResMut<ExtractedWireframeConfig>,
    colors_changed: Extract<
        Query<(Entity, &WireframeColor), (With<Wireframe>, Changed<WireframeColor>)>,
    >,
    without_colors: Extract<Query<Entity, (With<Wireframe>, Without<WireframeColor>)>>,
    no_wireframe_added: Extract<Query<Entity, Added<NoWireframe>>>,
    colors_removed: Extract<RemovedComponents<WireframeColor>>,
    no_wireframe_removed: Extract<RemovedComponents<NoWireframe>>,
    wireframe_removed: Extract<RemovedComponents<Wireframe>>,
    mut seen_colors: Local<HashSet<[f32; 4]>>,
) {
    seen_colors.clear();
    extracted_config.global = config.global;
    extracted_config.default_color = config.default_color.to_linear().to_f32_array();
    let default_color_entity = extracted_config
        .color_entities
        .entry(extracted_config.default_color)
        .or_insert_with(|| {
            commands.spawn(ExtractedWireframeColor {
                color: extracted_config.default_color,
            }).id()
        });
    seen_colors.insert(extracted_config.default_color);
    for (entity, wireframe_color) in &colors_changed {
        let linear_color = wireframe_color.color.to_linear().to_f32_array();
        let color_entity = extracted_config.color_entities.entry(linear_color).or_insert_with(|| {
            commands.spawn(ExtractedWireframeColor {
                color: linear_color,
            }).id()
        });
        extracted_config
            .color_instances
            .insert(entity.into(), *color_entity);
    }
    for entity in &no_wireframe_added {
        extracted_config
            .no_wireframe_instances
            .insert(entity.into());
    }
    for entity in no_wireframe_removed.iter() {
        extracted_config
            .no_wireframe_instances
            .remove(&entity.into());
    }
    for entity in colors_removed.iter() {
        if without_colors.contains(entity) {
            extracted_config
                .color_instances
                .insert(&entity.into(), extracted_config.default_color);
        } else {
            extracted_config.color_instances.remove(&entity.into());
        }
    }
    for entity in wireframe_removed.iter() {
        extracted_config.color_instances.remove(&entity.into());
    }
}

fn extract_wireframe_3d_camera(
    mut wireframe_3d_phases: ResMut<ViewBinnedRenderPhases<Wireframe3d>>,
    cameras: Extract<Query<(Entity,
                            &Camera,
                            Has<NoIndirectDrawing>,
    ), With<Camera3d>>>,
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

fn queue_wireframe(
    custom_draw_functions: Res<DrawFunctions<Wireframe3d>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<Wireframe3dPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    wireframe_3d_pipeline: Res<Wireframe3dPipeline>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    mut wireframe_3d_phases: ResMut<ViewBinnedRenderPhases<Wireframe3d>>,
    mut views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa)>,
    extracted_wireframe_config: Res<ExtractedWireframeConfig>,
) {
    for (view, visible_entities, msaa) in &mut views {
        let Some(wireframe_phase) = wireframe_3d_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let draw_custom = custom_draw_functions.read().id::<DrawWireframe3d>();

        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        for (render_entity, visible_entity) in visible_entities.iter::<Mesh3d>() {
            let Some(color) = extracted_wireframe_config.color_for_entity(*visible_entity) else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
            let pipeline_key = Wireframe3dPipelineKey {
                mesh_key,
                color,
            };

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &wireframe_3d_pipeline,
                pipeline_key,
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };
            let bin_key = Wireframe3dBinKey {
                color,
            };
            let batch_set_key = Wireframe3dBatchSetKey {
                vertex_slab: mesh_instance.vertex_slab,
                index_slab: mesh_instance.index_slab,
            };
            wireframe_phase.add(Wireframe3dBinKey {
                color_entity: (),
            });
        }
    }
}