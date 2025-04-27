use super::{meshlet_mesh_manager::MeshletMeshManager, MeshletMesh, MeshletMesh3d};
use crate::{
    material::DUMMY_MESH_MATERIAL, Material, MaterialBindingId, MeshFlags, MeshTransforms,
    MeshUniform, NotShadowCaster, NotShadowReceiver, PreviousGlobalTransform,
    RenderMaterialBindings, RenderMaterialInstances,
};
use bevy_asset::{AssetEvent, AssetServer, Assets, UntypedAssetId};
use bevy_ecs::{
    entity::{Entities, Entity, EntityHashMap},
    event::EventReader,
    query::Has,
    resource::Resource,
    system::{Local, Query, Res, ResMut, SystemState},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_render::{
    render_resource::StorageBuffer, sync_world::MainEntity, view::RenderLayers, MainWorld,
};
use bevy_transform::components::GlobalTransform;
use core::ops::{DerefMut, Range};

/// Manages data for each entity with a [`MeshletMesh`].
#[derive(Resource)]
pub struct InstanceManager {
    /// Amount of instances in the scene.
    pub scene_instance_count: u32,
    /// Amount of clusters in the scene.
    pub scene_cluster_count: u32,

    /// Per-instance [`MainEntity`], [`RenderLayers`], and [`NotShadowCaster`].
    pub instances: Vec<(MainEntity, RenderLayers, bool)>,
    /// Per-instance [`MeshUniform`].
    pub instance_uniforms: StorageBuffer<Vec<MeshUniform>>,
    /// Per-instance material ID.
    pub instance_material_ids: StorageBuffer<Vec<u32>>,
    /// Per-instance count of meshlets in the instance's [`MeshletMesh`].
    pub instance_meshlet_counts: StorageBuffer<Vec<u32>>,
    /// Per-instance index to the start of the instance's slice of the meshlets buffer.
    pub instance_meshlet_slice_starts: StorageBuffer<Vec<u32>>,
    /// Per-view per-instance visibility bit. Used for [`RenderLayers`] and [`NotShadowCaster`] support.
    pub view_instance_visibility: EntityHashMap<StorageBuffer<Vec<u32>>>,

    /// Next material ID available for a [`Material`].
    next_material_id: u32,
    /// Map of [`Material`] to material ID.
    material_id_lookup: HashMap<UntypedAssetId, u32>,
    /// Set of material IDs used in the scene.
    material_ids_present_in_scene: HashSet<u32>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            scene_instance_count: 0,
            scene_cluster_count: 0,

            instances: Vec::new(),
            instance_uniforms: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_uniforms"));
                buffer
            },
            instance_material_ids: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_material_ids"));
                buffer
            },
            instance_meshlet_counts: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_meshlet_counts"));
                buffer
            },
            instance_meshlet_slice_starts: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_meshlet_slice_starts"));
                buffer
            },
            view_instance_visibility: EntityHashMap::default(),

            next_material_id: 0,
            material_id_lookup: HashMap::default(),
            material_ids_present_in_scene: HashSet::default(),
        }
    }

    pub fn add_instance(
        &mut self,
        instance: MainEntity,
        meshlets_slice: Range<u32>,
        transform: &GlobalTransform,
        previous_transform: Option<&PreviousGlobalTransform>,
        render_layers: Option<&RenderLayers>,
        mesh_material_ids: &RenderMaterialInstances,
        render_material_bindings: &RenderMaterialBindings,
        not_shadow_receiver: bool,
        not_shadow_caster: bool,
    ) {
        // Build a MeshUniform for the instance
        let transform = transform.affine();
        let previous_transform = previous_transform.map(|t| t.0).unwrap_or(transform);
        let mut flags = if not_shadow_receiver {
            MeshFlags::empty()
        } else {
            MeshFlags::SHADOW_RECEIVER
        };
        if transform.matrix3.determinant().is_sign_positive() {
            flags |= MeshFlags::SIGN_DETERMINANT_MODEL_3X3;
        }
        let transforms = MeshTransforms {
            world_from_local: (&transform).into(),
            previous_world_from_local: (&previous_transform).into(),
            flags: flags.bits(),
        };

        let mesh_material = mesh_material_ids.mesh_material(instance);
        let mesh_material_binding_id = if mesh_material != DUMMY_MESH_MATERIAL.untyped() {
            render_material_bindings
                .get(&mesh_material)
                .cloned()
                .unwrap_or_default()
        } else {
            // Use a dummy binding ID if the mesh has no material
            MaterialBindingId::default()
        };

        let mesh_uniform = MeshUniform::new(
            &transforms,
            0,
            mesh_material_binding_id.slot,
            None,
            None,
            None,
        );

        // Append instance data
        self.instances.push((
            instance,
            render_layers.cloned().unwrap_or(RenderLayers::default()),
            not_shadow_caster,
        ));
        self.instance_uniforms.get_mut().push(mesh_uniform);
        self.instance_material_ids.get_mut().push(0);
        self.instance_meshlet_counts
            .get_mut()
            .push(meshlets_slice.len() as u32);
        self.instance_meshlet_slice_starts
            .get_mut()
            .push(meshlets_slice.start);

        self.scene_instance_count += 1;
        self.scene_cluster_count += meshlets_slice.len() as u32;
    }

    /// Get the material ID for a [`crate::Material`].
    pub fn get_material_id(&mut self, material_asset_id: UntypedAssetId) -> u32 {
        *self
            .material_id_lookup
            .entry(material_asset_id)
            .or_insert_with(|| {
                self.next_material_id += 1;
                self.next_material_id
            })
    }

    pub fn material_present_in_scene(&self, material_id: &u32) -> bool {
        self.material_ids_present_in_scene.contains(material_id)
    }

    pub fn reset(&mut self, entities: &Entities) {
        self.scene_instance_count = 0;
        self.scene_cluster_count = 0;

        self.instances.clear();
        self.instance_uniforms.get_mut().clear();
        self.instance_material_ids.get_mut().clear();
        self.instance_meshlet_counts.get_mut().clear();
        self.instance_meshlet_slice_starts.get_mut().clear();
        self.view_instance_visibility
            .retain(|view_entity, _| entities.contains(*view_entity));
        self.view_instance_visibility
            .values_mut()
            .for_each(|b| b.get_mut().clear());

        self.next_material_id = 0;
        self.material_id_lookup.clear();
        self.material_ids_present_in_scene.clear();
    }
}

pub fn extract_meshlet_mesh_entities(
    mut meshlet_mesh_manager: ResMut<MeshletMeshManager>,
    mut instance_manager: ResMut<InstanceManager>,
    // TODO: Replace main_world and system_state when Extract<ResMut<Assets<MeshletMesh>>> is possible
    mut main_world: ResMut<MainWorld>,
    mesh_material_ids: Res<RenderMaterialInstances>,
    render_material_bindings: Res<RenderMaterialBindings>,
    mut system_state: Local<
        Option<
            SystemState<(
                Query<(
                    Entity,
                    &MeshletMesh3d,
                    &GlobalTransform,
                    Option<&PreviousGlobalTransform>,
                    Option<&RenderLayers>,
                    Has<NotShadowReceiver>,
                    Has<NotShadowCaster>,
                )>,
                Res<AssetServer>,
                ResMut<Assets<MeshletMesh>>,
                EventReader<AssetEvent<MeshletMesh>>,
            )>,
        >,
    >,
    render_entities: &Entities,
) {
    // Get instances query
    if system_state.is_none() {
        *system_state = Some(SystemState::new(&mut main_world));
    }
    let system_state = system_state.as_mut().unwrap();
    let (instances_query, asset_server, mut assets, mut asset_events) =
        system_state.get_mut(&mut main_world);

    // Reset per-frame data
    instance_manager.reset(render_entities);

    // Free GPU buffer space for any modified or dropped MeshletMesh assets
    for asset_event in asset_events.read() {
        if let AssetEvent::Unused { id } | AssetEvent::Modified { id } = asset_event {
            meshlet_mesh_manager.remove(id);
        }
    }

    // Iterate over every instance
    for (
        instance,
        meshlet_mesh,
        transform,
        previous_transform,
        render_layers,
        not_shadow_receiver,
        not_shadow_caster,
    ) in &instances_query
    {
        // Skip instances with an unloaded MeshletMesh asset
        // TODO: This is a semi-expensive check
        if asset_server.is_managed(meshlet_mesh.id())
            && !asset_server.is_loaded_with_dependencies(meshlet_mesh.id())
        {
            continue;
        }

        // Upload the instance's MeshletMesh asset data if not done already done
        let meshlets_slice =
            meshlet_mesh_manager.queue_upload_if_needed(meshlet_mesh.id(), &mut assets);

        // Add the instance's data to the instance manager
        instance_manager.add_instance(
            instance.into(),
            meshlets_slice,
            transform,
            previous_transform,
            render_layers,
            &mesh_material_ids,
            &render_material_bindings,
            not_shadow_receiver,
            not_shadow_caster,
        );
    }
}

/// For each entity in the scene, record what material ID its material was assigned in the `prepare_material_meshlet_meshes` systems,
/// and note that the material is used by at least one entity in the scene.
pub fn queue_material_meshlet_meshes<M: Material>(
    mut instance_manager: ResMut<InstanceManager>,
    render_material_instances: Res<RenderMaterialInstances>,
) {
    let instance_manager = instance_manager.deref_mut();

    for (i, (instance, _, _)) in instance_manager.instances.iter().enumerate() {
        if let Some(material_instance) = render_material_instances.instances.get(instance) {
            if let Ok(material_asset_id) = material_instance.asset_id.try_typed::<M>() {
                if let Some(material_id) = instance_manager
                    .material_id_lookup
                    .get(&material_asset_id.untyped())
                {
                    instance_manager
                        .material_ids_present_in_scene
                        .insert(*material_id);
                    instance_manager.instance_material_ids.get_mut()[i] = *material_id;
                }
            }
        }
    }
}
