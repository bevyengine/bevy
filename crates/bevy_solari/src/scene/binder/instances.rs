use super::{
    allocator::{IndexAllocator, RetainedBindingArray},
    assets::AssetState,
    lights::{GpuLightSource, LightSourceId, LightState},
    BlasManager, RaytracingMesh3d, RaytracingSceneBindings,
};
use bevy_asset::AssetId;
use bevy_ecs::{
    entity::{Entity, EntityHashMap, EntityHashSet},
    query::{Changed, Or, With},
    system::Query,
};
use bevy_math::{Affine3, Affine3Ext, Vec4};
use bevy_mesh::Mesh;
use bevy_pbr::{MeshMaterial3d, PreviousGlobalTransform, StandardMaterial};
use bevy_platform::collections::HashMap;
use bevy_render::{
    impl_atomic_pod,
    mesh::allocator::MeshAllocator,
    render_resource::{AtomicPod, AtomicSparseBufferVec, Buffer, BufferId, BufferUsages},
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::once;
use bytemuck::{Pod, Zeroable};
use core::{hash::Hash, num::NonZeroU32};
use tracing::{info_span, warn};

pub const MAX_MESH_SLAB_COUNT: NonZeroU32 = NonZeroU32::new(500).unwrap();

#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GpuInstanceGeometryIds {
    vertex_buffer_id: u32,
    vertex_buffer_offset: u32,
    index_buffer_id: u32,
    index_buffer_offset: u32,
    triangle_count: u32,
}

/// A world-from-local affine transform, stored transposed as three rows.
#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GpuTransform([Vec4; 3]);

impl GpuTransform {
    /// The three rows as the flat row-major 3x4 a [`TlasInstance`] wants.
    ///
    /// [`TlasInstance`]: bevy_render::render_resource::TlasInstance
    fn rows(self) -> [f32; 12] {
        bytemuck::cast(self)
    }
}

/// The device address of a slot's acceleration structure. Zero marks an inactive slot.
#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct GpuBlasRef(u64);

impl GpuBlasRef {
    const NONE: Self = Self(0);
}

impl_atomic_pod!(GpuInstanceGeometryIds, GpuInstanceGeometryIdsBlob);
impl_atomic_pod!(GpuTransform, GpuTransformBlob);
impl_atomic_pod!(GpuBlasRef, GpuBlasRefBlob);

fn storage_buffer<T: AtomicPod>(label: &str) -> AtomicSparseBufferVec<T> {
    AtomicSparseBufferVec::new(BufferUsages::STORAGE, label.into())
}

/// Everything tracked per raytracing instance.
#[derive(Clone)]
struct Instance {
    slot: u32,
    mesh: AssetId<Mesh>,
    material: AssetId<StandardMaterial>,
    buffers: Option<(BufferId, BufferId)>,
}

/// Stable slots, reverse dependency indices and GPU data owned by raytracing instances.
pub struct InstanceState {
    pub vertex_buffers: RetainedBindingArray<BufferId, Buffer>,
    pub index_buffers: RetainedBindingArray<BufferId, Buffer>,
    pub transforms: AtomicSparseBufferVec<GpuTransform>,
    pub previous_frame_transforms: AtomicSparseBufferVec<GpuTransform>,
    pub geometry_ids: AtomicSparseBufferVec<GpuInstanceGeometryIds>,
    pub material_ids: AtomicSparseBufferVec<u32>,
    pub blas_refs: AtomicSparseBufferVec<GpuBlasRef>,
    pub slots: IndexAllocator,
    records: EntityHashMap<Instance>,
    pub live_count: u32,
    pub pending_refresh: EntityHashSet,
    mesh_instances: HashMap<AssetId<Mesh>, EntityHashSet>,
    pub material_instances: HashMap<AssetId<StandardMaterial>, EntityHashSet>,
}

impl InstanceState {
    pub fn new() -> Self {
        Self {
            vertex_buffers: RetainedBindingArray::new(),
            index_buffers: RetainedBindingArray::new(),
            transforms: storage_buffer("solari_transforms"),
            previous_frame_transforms: storage_buffer("solari_previous_frame_transforms"),
            geometry_ids: storage_buffer("solari_geometry_ids"),
            material_ids: storage_buffer("solari_material_ids"),
            blas_refs: storage_buffer("solari_blas_refs"),
            slots: IndexAllocator::new(),
            records: EntityHashMap::default(),
            live_count: 0,
            pending_refresh: EntityHashSet::default(),
            mesh_instances: HashMap::default(),
            material_instances: HashMap::default(),
        }
    }

    /// Every drawable instance's slot, mesh and world-from-local transform.
    ///
    /// Only the `wgpu-core` TLAS build path needs this, to fill in the instance descriptors that
    /// the raw path sets up on the GPU. Slots with a null acceleration structure reference are not
    /// currently drawable, and are left out.
    pub fn drawable(&self) -> impl Iterator<Item = (u32, AssetId<Mesh>, [f32; 12])> + '_ {
        self.records.values().filter_map(|instance| {
            let slot = instance.slot;
            (self.blas_refs.get(slot) != GpuBlasRef::NONE)
                .then(|| (slot, instance.mesh, self.transforms.get(slot).rows()))
        })
    }

    /// Queues every instance using `material_id` to be re-resolved.
    pub fn invalidate_material(&mut self, material_id: AssetId<StandardMaterial>) {
        if let Some(instances) = self.material_instances.get(&material_id) {
            self.pending_refresh.extend(instances.iter().copied());
        }
    }
}

pub type InstanceQueryData<'w> = (
    &'w RaytracingMesh3d,
    &'w MeshMaterial3d<StandardMaterial>,
    &'w GlobalTransform,
    &'w PreviousGlobalTransform,
);

pub type ChangedInstanceFilter = (
    With<RaytracingMesh3d>,
    Or<(
        Changed<RaytracingMesh3d>,
        Changed<MeshMaterial3d<StandardMaterial>>,
    )>,
);

/// The scene state an instance resolves its GPU data against.
pub struct InstanceInputs<'a> {
    pub assets: &'a AssetState,
    pub blas_manager: &'a BlasManager,
    pub mesh_allocator: &'a MeshAllocator,
}

fn unlink<K: Eq + Hash>(map: &mut HashMap<K, EntityHashSet>, key: &K, entity: Entity) {
    let now_empty = map.get_mut(key).is_some_and(|instances| {
        instances.remove(&entity);
        instances.is_empty()
    });
    if now_empty {
        map.remove(key);
    }
}

fn relink<K: Copy + Eq + Hash>(
    map: &mut HashMap<K, EntityHashSet>,
    entity: Entity,
    previous: Option<K>,
    key: K,
) {
    if previous == Some(key) {
        return;
    }
    if let Some(previous) = previous {
        unlink(map, &previous, entity);
    }
    map.entry(key).or_default().insert(entity);
}

impl InstanceState {
    pub fn remove_instances(
        &mut self,
        lights: &mut LightState,
        removed: impl IntoIterator<Item = Entity>,
    ) {
        let _span = info_span!("remove_instances").entered();
        for entity in removed {
            self.remove_instance(lights, entity);
        }
    }

    pub fn refresh_instances(
        &mut self,
        inputs: &InstanceInputs,
        lights: &mut LightState,
        instances: &Query<InstanceQueryData>,
        changed_instances: &Query<Entity, ChangedInstanceFilter>,
    ) {
        let _span = info_span!("refresh_instances").entered();

        let mut refresh = core::mem::take(&mut self.pending_refresh);
        refresh.extend(changed_instances.iter());

        let moved_meshes = inputs.mesh_allocator.meshes_displaced_by_slab_growth();
        for mesh_id in inputs
            .blas_manager
            .changed_meshes()
            .iter()
            .copied()
            .chain(moved_meshes)
        {
            if let Some(mesh_instances) = self.mesh_instances.get(&mesh_id) {
                refresh.extend(mesh_instances.iter().copied());
            }
        }

        for entity in refresh {
            match instances.get(entity) {
                Ok(data) => self.refresh_instance(inputs, lights, entity, data),
                Err(_) => self.remove_instance(lights, entity),
            }
        }
    }

    fn reserve_slot(&mut self, slot: u32) {
        let len = slot + 1;
        self.transforms.grow(len);
        self.previous_frame_transforms.grow(len);
        self.blas_refs.grow(len);
    }

    fn refresh_instance(
        &mut self,
        inputs: &InstanceInputs,
        lights: &mut LightState,
        entity: Entity,
        (mesh, material, transform, previous_frame_transform): InstanceQueryData,
    ) {
        let mesh_id = mesh.id();
        let material_id = material.id();
        let previous = self.records.get(&entity).cloned();

        relink(
            &mut self.mesh_instances,
            entity,
            previous.as_ref().map(|instance| instance.mesh),
            mesh_id,
        );
        relink(
            &mut self.material_instances,
            entity,
            previous.as_ref().map(|instance| instance.material),
            material_id,
        );

        let slot = match &previous {
            Some(previous) => previous.slot,
            None => self.slots.allocate(),
        };
        self.reserve_slot(slot);

        // Seed only once. Later refreshes must not overwrite transforms written by extraction.
        if previous.is_none() {
            self.write_transforms(slot, transform, previous_frame_transform);
        }

        let mut instance = Instance {
            slot,
            mesh: mesh_id,
            material: material_id,
            buffers: previous.and_then(|instance| instance.buffers),
        };
        let resolved = self.resolve_instance(inputs, lights, entity, &mut instance);

        self.records.insert(entity, instance);
        if !resolved {
            self.pending_refresh.insert(entity);
        }
    }

    fn resolve_instance(
        &mut self,
        inputs: &InstanceInputs,
        lights: &mut LightState,
        entity: Entity,
        instance: &mut Instance,
    ) -> bool {
        let slot = instance.slot;
        let (Some(vertex_slice), Some(index_slice), Some(material_slot), Some(blas_address)) = (
            inputs.mesh_allocator.mesh_vertex_slice(&instance.mesh),
            inputs.mesh_allocator.mesh_index_slice(&instance.mesh),
            inputs.assets.material_slots.get(&instance.material),
            inputs.blas_manager.device_address(&instance.mesh),
        ) else {
            self.deactivate_instance(lights, entity, instance);
            return false;
        };

        let vertex_buffer_key = vertex_slice.buffer.id();
        let index_buffer_key = index_slice.buffer.id();
        let capacity = MAX_MESH_SLAB_COUNT.get();
        if !self.vertex_buffers.has_room(&vertex_buffer_key, capacity)
            || !self.index_buffers.has_room(&index_buffer_key, capacity)
        {
            once!(warn!(
                "Solari scene needs more than {} mesh slabs. Instances past that limit will \
                 not be rendered.",
                MAX_MESH_SLAB_COUNT.get()
            ));
            self.deactivate_instance(lights, entity, instance);
            return false;
        }

        let previous_buffers = instance.buffers.take();
        let vertex_buffer_id = self
            .vertex_buffers
            .acquire(vertex_buffer_key.clone(), capacity, || {
                vertex_slice.buffer.clone()
            })
            .expect("vertex slab binding array had room but handed out no slot");
        let index_buffer_id = self
            .index_buffers
            .acquire(index_buffer_key.clone(), capacity, || {
                index_slice.buffer.clone()
            })
            .expect("index slab binding array had room but handed out no slot");
        instance.buffers = Some((vertex_buffer_key, index_buffer_key));
        self.release_buffers(previous_buffers);

        let triangle_count = (index_slice.range.len() / 3) as u32;
        self.geometry_ids.grow_and_set(
            slot,
            GpuInstanceGeometryIds {
                vertex_buffer_id,
                vertex_buffer_offset: vertex_slice.range.start,
                index_buffer_id,
                index_buffer_offset: index_slice.range.start,
                triangle_count,
            },
        );
        self.material_ids.grow_and_set(slot, material_slot);
        self.set_blas_ref(slot, GpuBlasRef(blas_address));

        let is_emissive = inputs
            .assets
            .emissive_materials
            .contains(&instance.material);
        if is_emissive {
            lights.add_light(
                LightSourceId::EmissiveMesh(entity),
                GpuLightSource::new_emissive_mesh_light(slot, triangle_count),
            );
        } else {
            lights.remove_light(LightSourceId::EmissiveMesh(entity));
        }
        true
    }

    fn write_transforms(
        &self,
        slot: u32,
        transform: &GlobalTransform,
        previous_frame_transform: &PreviousGlobalTransform,
    ) {
        self.transforms.set_if_changed(
            slot,
            GpuTransform(Affine3::from(transform.affine()).to_transpose()),
        );
        self.previous_frame_transforms.set_if_changed(
            slot,
            GpuTransform(Affine3::from(previous_frame_transform.0).to_transpose()),
        );
    }

    /// Points a slot at an acceleration structure, or at nothing, keeping `live_count` in step.
    fn set_blas_ref(&mut self, slot: u32, reference: GpuBlasRef) {
        self.blas_refs.grow(slot + 1);
        let previous = self.blas_refs.get(slot);
        if previous == reference {
            return;
        }
        self.blas_refs.set(slot, reference);

        if previous == GpuBlasRef::NONE {
            self.live_count += 1;
        } else if reference == GpuBlasRef::NONE {
            self.live_count -= 1;
        }
    }

    fn deactivate_instance(
        &mut self,
        lights: &mut LightState,
        entity: Entity,
        instance: &mut Instance,
    ) {
        self.set_blas_ref(instance.slot, GpuBlasRef::NONE);
        lights.remove_light(LightSourceId::EmissiveMesh(entity));
        self.release_buffers(instance.buffers.take());
    }

    fn release_buffers(&mut self, buffers: Option<(BufferId, BufferId)>) {
        if let Some((vertex_key, index_key)) = buffers {
            self.vertex_buffers.release(&vertex_key);
            self.index_buffers.release(&index_key);
        }
    }

    fn remove_instance(&mut self, lights: &mut LightState, entity: Entity) {
        let Some(mut instance) = self.records.remove(&entity) else {
            return;
        };

        self.deactivate_instance(lights, entity, &mut instance);
        self.slots.release(instance.slot);
        self.pending_refresh.remove(&entity);
        unlink(&mut self.mesh_instances, &instance.mesh, entity);
        unlink(&mut self.material_instances, &instance.material, entity);
    }
}

impl RaytracingSceneBindings {
    /// Parallel hot path: one entity lookup, then two allocation-free sparse writes.
    pub fn move_instance(
        &self,
        entity: Entity,
        transform: &GlobalTransform,
        previous_frame_transform: &PreviousGlobalTransform,
    ) {
        if let Some(instance) = self.instances.records.get(&entity) {
            self.instances
                .write_transforms(instance.slot, transform, previous_frame_transform);
        }
    }
}
