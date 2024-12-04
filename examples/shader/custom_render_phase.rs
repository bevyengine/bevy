use std::ops::Range;

use bevy::{
    asset::UntypedAssetId,
    core_pipeline::{
        core_2d::Opaque2dBinKey,
        tonemapping::{DebandDither, Tonemapping},
    },
    ecs::entity::{EntityHash, EntityHashSet},
    pbr::tonemapping_pipeline_key,
    prelude::*,
    sprite::{Material2dKey, Mesh2dPipelineKey, RenderMesh2dInstances},
};
use bevy_render::{
    mesh::RenderMesh,
    render_asset::RenderAssets,
    render_phase::{
        BinnedPhaseItem, BinnedRenderPhaseType, CachedRenderPipelinePhaseItem, DrawFunctionId,
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, ViewBinnedRenderPhases,
    },
    render_resource::{BindGroupId, CachedRenderPipelineId, PipelineCache},
    sync_world::MainEntity,
    view::{ExtractedView, VisibleEntities},
    Extract, Render, RenderApp,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CustomPhasPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

struct CustomPhasPlugin;
impl Plugin for CustomPhasPlugin {
    fn build(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<DrawFunctions<CustomPhase>>()
            .init_resource::<ViewBinnedRenderPhases<CustomPhase>>()
            .add_systems(Render, extract_camera_phases);
        //.add_systems(
        //    Render,
        //    (batch_and_prepare_binned_render_phase::<CustomPhase>,),
        //);
    }
    fn finish(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
    }
}

// TODO Phase
// TODO Phase Item
// TODO ViewNode

pub struct CustomPhase {
    /// The key, which determines which can be batched.
    pub key: CustomPhaseBinKey,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub representative_entity: (Entity, MainEntity),
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

/// Data that must be identical in order to batch phase items together.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomPhaseBinKey {
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

impl PhaseItem for CustomPhase {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
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

impl BinnedPhaseItem for CustomPhase {
    type BinKey = CustomPhaseBinKey;

    fn new(
        key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        CustomPhase {
            key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for CustomPhase {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.key.pipeline
    }
}

fn extract_camera_phases(
    mut commands: Commands,
    mut custom_phases: ResMut<ViewBinnedRenderPhases<CustomPhase>>,
    cameras: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();
    for (entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }
        commands.get_or_spawn(entity);
        custom_phases.insert_or_clear(entity);
        live_entities.insert(entity);
    }
    // Clear out all dead views.
    custom_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

#[allow(clippy::too_many_arguments)]
pub fn queue_material2d_meshes(
    opaque_draw_functions: Res<DrawFunctions<CustomPhase>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    mut render_mesh_instances: ResMut<RenderMesh2dInstances>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<CustomPhase>>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &VisibleEntities,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
) {
    if render_material_instances.is_empty() {
        return;
    }
    for (view_entity, view, visible_entities, msaa, tonemapping, dither) in &mut views {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view_entity) else {
            continue;
        };
        let draw_opaque_2d = opaque_draw_functions.read().id::<DrawMaterial2d<M>>();
        let mut view_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);
        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= Mesh2dPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= Mesh2dPipelineKey::DEBAND_DITHER;
            }
        }
        for visible_entity in visible_entities.iter::<WithMesh2d>() {
            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.get_mut(visible_entity) else {
                continue;
            };
            let Some(material_2d) = render_materials.get(*material_asset_id) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let mesh_key = view_key
                | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology())
                | material_2d.properties.mesh_pipeline_key_bits;
            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &material2d_pipeline,
                Material2dKey {
                    mesh_key,
                    bind_group_data: material_2d.key.clone(),
                },
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };
            mesh_instance.material_bind_group_id = material_2d.get_bind_group_id();
            let mesh_z = mesh_instance.transforms.world_from_local.translation.z;

            let bin_key = Opaque2dBinKey {
                pipeline: pipeline_id,
                draw_function: draw_opaque_2d,
                asset_id: mesh_instance.mesh_asset_id.into(),
                material_bind_group_id: material_2d.get_bind_group_id().0,
            };
            opaque_phase.add(
                bin_key,
                *visible_entity,
                BinnedRenderPhaseType::mesh(mesh_instance.automatic_batching),
            );
        }
    }
}
