use core::num::NonZero;

use bevy_camera::Camera;
use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_light::cluster::{ClusterableObjectCounts, Clusters, GlobalClusterSettings};
use bevy_math::{uvec4, UVec3, UVec4, Vec4};
use bevy_render::{
    render_resource::{
        BindingResource, BufferBindingType, ShaderSize, ShaderType, StorageBuffer, UniformBuffer,
    },
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    Extract,
};
use tracing::warn;

use crate::MeshPipeline;

// NOTE: this must be kept in sync with the same constants in
// `mesh_view_types.wgsl`.
pub const MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS: usize = 204;
// Make sure that the clusterable object buffer doesn't overflow the maximum
// size of a UBO on WebGL 2.
const _: () =
    assert!(size_of::<GpuClusterableObject>() * MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS <= 16384);

// NOTE: Clustered-forward rendering requires 3 storage buffer bindings so check that
// at least that many are supported using this constant and SupportedBindingType::from_device()
pub const CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT: u32 = 3;

// this must match CLUSTER_COUNT_SIZE in pbr.wgsl
// and must be large enough to contain MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS
const CLUSTER_COUNT_SIZE: u32 = 9;

const CLUSTER_OFFSET_MASK: u32 = (1 << (32 - (CLUSTER_COUNT_SIZE * 2))) - 1;
const CLUSTER_COUNT_MASK: u32 = (1 << CLUSTER_COUNT_SIZE) - 1;

pub(crate) fn make_global_cluster_settings(world: &World) -> GlobalClusterSettings {
    let device = world.resource::<RenderDevice>();
    let adapter = world.resource::<RenderAdapter>();
    let clustered_decals_are_usable =
        crate::decal::clustered::clustered_decals_are_usable(device, adapter);
    let supports_storage_buffers = matches!(
        device.get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT),
        BufferBindingType::Storage { .. }
    );
    GlobalClusterSettings {
        supports_storage_buffers,
        clustered_decals_are_usable,
        max_uniform_buffer_clusterable_objects: MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS,
        view_cluster_bindings_max_indices: ViewClusterBindings::MAX_INDICES,
    }
}

#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuClusterableObject {
    // For point lights: the lower-right 2x2 values of the projection matrix [2][2] [2][3] [3][2] [3][3]
    // For spot lights: 2 components of the direction (x,z), spot_scale and spot_offset
    pub(crate) light_custom_data: Vec4,
    pub(crate) color_inverse_square_range: Vec4,
    pub(crate) position_radius: Vec4,
    pub(crate) flags: u32,
    pub(crate) shadow_depth_bias: f32,
    pub(crate) shadow_normal_bias: f32,
    pub(crate) spot_light_tan_angle: f32,
    pub(crate) soft_shadow_size: f32,
    pub(crate) shadow_map_near_z: f32,
    pub(crate) decal_index: u32,
    pub(crate) pad: f32,
}

#[derive(Resource)]
pub struct GlobalClusterableObjectMeta {
    pub gpu_clusterable_objects: GpuClusterableObjects,
    pub entity_to_index: EntityHashMap<usize>,
}

pub enum GpuClusterableObjects {
    Uniform(UniformBuffer<GpuClusterableObjectsUniform>),
    Storage(StorageBuffer<GpuClusterableObjectsStorage>),
}

#[derive(ShaderType)]
pub struct GpuClusterableObjectsUniform {
    data: Box<[GpuClusterableObject; MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS]>,
}

#[derive(ShaderType, Default)]
pub struct GpuClusterableObjectsStorage {
    #[size(runtime)]
    data: Vec<GpuClusterableObject>,
}

#[derive(Component)]
pub struct ExtractedClusterConfig {
    /// Special near value for cluster calculations
    pub(crate) near: f32,
    pub(crate) far: f32,
    /// Number of clusters in `X` / `Y` / `Z` in the view frustum
    pub(crate) dimensions: UVec3,
}

enum ExtractedClusterableObjectElement {
    ClusterHeader(ClusterableObjectCounts),
    ClusterableObjectEntity(Entity),
}

#[derive(Component)]
pub struct ExtractedClusterableObjects {
    data: Vec<ExtractedClusterableObjectElement>,
}

#[derive(ShaderType)]
struct GpuClusterOffsetsAndCountsUniform {
    data: Box<[UVec4; ViewClusterBindings::MAX_UNIFORM_ITEMS]>,
}

#[derive(ShaderType, Default)]
struct GpuClusterableObjectIndexListsStorage {
    #[size(runtime)]
    data: Vec<u32>,
}

#[derive(ShaderType, Default)]
struct GpuClusterOffsetsAndCountsStorage {
    /// The starting offset, followed by the number of point lights, spot
    /// lights, reflection probes, and irradiance volumes in each cluster, in
    /// that order. The remaining fields are filled with zeroes.
    #[size(runtime)]
    data: Vec<[UVec4; 2]>,
}

enum ViewClusterBuffers {
    Uniform {
        // NOTE: UVec4 is because all arrays in Std140 layout have 16-byte alignment
        clusterable_object_index_lists: UniformBuffer<GpuClusterableObjectIndexListsUniform>,
        // NOTE: UVec4 is because all arrays in Std140 layout have 16-byte alignment
        cluster_offsets_and_counts: UniformBuffer<GpuClusterOffsetsAndCountsUniform>,
    },
    Storage {
        clusterable_object_index_lists: StorageBuffer<GpuClusterableObjectIndexListsStorage>,
        cluster_offsets_and_counts: StorageBuffer<GpuClusterOffsetsAndCountsStorage>,
    },
}

#[derive(Component)]
pub struct ViewClusterBindings {
    n_indices: usize,
    n_offsets: usize,
    buffers: ViewClusterBuffers,
}

impl FromWorld for GlobalClusterableObjectMeta {
    fn from_world(world: &mut World) -> Self {
        Self::new(
            world
                .resource::<RenderDevice>()
                .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT),
        )
    }
}

impl GlobalClusterableObjectMeta {
    pub fn new(buffer_binding_type: BufferBindingType) -> Self {
        Self {
            gpu_clusterable_objects: GpuClusterableObjects::new(buffer_binding_type),
            entity_to_index: EntityHashMap::default(),
        }
    }
}

impl GpuClusterableObjects {
    fn new(buffer_binding_type: BufferBindingType) -> Self {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => Self::storage(),
            BufferBindingType::Uniform => Self::uniform(),
        }
    }

    fn uniform() -> Self {
        Self::Uniform(UniformBuffer::default())
    }

    fn storage() -> Self {
        Self::Storage(StorageBuffer::default())
    }

    pub(crate) fn set(&mut self, mut clusterable_objects: Vec<GpuClusterableObject>) {
        match self {
            GpuClusterableObjects::Uniform(buffer) => {
                let len = clusterable_objects
                    .len()
                    .min(MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS);
                let src = &clusterable_objects[..len];
                let dst = &mut buffer.get_mut().data[..len];
                dst.copy_from_slice(src);
            }
            GpuClusterableObjects::Storage(buffer) => {
                buffer.get_mut().data.clear();
                buffer.get_mut().data.append(&mut clusterable_objects);
            }
        }
    }

    pub(crate) fn write_buffer(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        match self {
            GpuClusterableObjects::Uniform(buffer) => {
                buffer.write_buffer(render_device, render_queue);
            }
            GpuClusterableObjects::Storage(buffer) => {
                buffer.write_buffer(render_device, render_queue);
            }
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            GpuClusterableObjects::Uniform(buffer) => buffer.binding(),
            GpuClusterableObjects::Storage(buffer) => buffer.binding(),
        }
    }

    pub fn min_size(buffer_binding_type: BufferBindingType) -> NonZero<u64> {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => GpuClusterableObjectsStorage::min_size(),
            BufferBindingType::Uniform => GpuClusterableObjectsUniform::min_size(),
        }
    }
}

impl Default for GpuClusterableObjectsUniform {
    fn default() -> Self {
        Self {
            data: Box::new(
                [GpuClusterableObject::default(); MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS],
            ),
        }
    }
}

/// Extracts clusters from the main world from the render world.
pub fn extract_clusters(
    mut commands: Commands,
    views: Extract<Query<(RenderEntity, &Clusters, &Camera)>>,
    mapper: Extract<Query<RenderEntity>>,
) {
    for (entity, clusters, camera) in &views {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Clusters entity wasn't synced.");
        if !camera.is_active {
            entity_commands.remove::<(ExtractedClusterableObjects, ExtractedClusterConfig)>();
            continue;
        }

        let entity_count: usize = clusters
            .clusterable_objects
            .iter()
            .map(|l| l.entities.len())
            .sum();
        let mut data = Vec::with_capacity(clusters.clusterable_objects.len() + entity_count);
        for cluster_objects in &clusters.clusterable_objects {
            data.push(ExtractedClusterableObjectElement::ClusterHeader(
                cluster_objects.counts,
            ));
            for clusterable_entity in &cluster_objects.entities {
                if let Ok(entity) = mapper.get(*clusterable_entity) {
                    data.push(ExtractedClusterableObjectElement::ClusterableObjectEntity(
                        entity,
                    ));
                }
            }
        }

        entity_commands.insert((
            ExtractedClusterableObjects { data },
            ExtractedClusterConfig {
                near: clusters.near,
                far: clusters.far,
                dimensions: clusters.dimensions,
            },
        ));
    }
}

pub fn prepare_clusters(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mesh_pipeline: Res<MeshPipeline>,
    global_clusterable_object_meta: Res<GlobalClusterableObjectMeta>,
    views: Query<(Entity, &ExtractedClusterableObjects)>,
) {
    let render_device = render_device.into_inner();
    let supports_storage_buffers = matches!(
        mesh_pipeline.clustered_forward_buffer_binding_type,
        BufferBindingType::Storage { .. }
    );
    for (entity, extracted_clusters) in &views {
        let mut view_clusters_bindings =
            ViewClusterBindings::new(mesh_pipeline.clustered_forward_buffer_binding_type);
        view_clusters_bindings.clear();

        for record in &extracted_clusters.data {
            match record {
                ExtractedClusterableObjectElement::ClusterHeader(counts) => {
                    let offset = view_clusters_bindings.n_indices();
                    view_clusters_bindings.push_offset_and_counts(offset, counts);
                }
                ExtractedClusterableObjectElement::ClusterableObjectEntity(entity) => {
                    if let Some(clusterable_object_index) =
                        global_clusterable_object_meta.entity_to_index.get(entity)
                    {
                        if view_clusters_bindings.n_indices() >= ViewClusterBindings::MAX_INDICES
                            && !supports_storage_buffers
                        {
                            warn!(
                                "Clusterable object index lists are full! The clusterable \
                                 objects in the view are present in too many clusters."
                            );
                            break;
                        }
                        view_clusters_bindings.push_index(*clusterable_object_index);
                    }
                }
            }
        }

        view_clusters_bindings.write_buffers(render_device, &render_queue);

        commands.entity(entity).insert(view_clusters_bindings);
    }
}

impl ViewClusterBindings {
    pub const MAX_OFFSETS: usize = 16384 / 4;
    const MAX_UNIFORM_ITEMS: usize = Self::MAX_OFFSETS / 4;
    pub const MAX_INDICES: usize = 16384;

    pub fn new(buffer_binding_type: BufferBindingType) -> Self {
        Self {
            n_indices: 0,
            n_offsets: 0,
            buffers: ViewClusterBuffers::new(buffer_binding_type),
        }
    }

    pub fn clear(&mut self) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                clusterable_object_index_lists,
                cluster_offsets_and_counts,
            } => {
                *clusterable_object_index_lists.get_mut().data =
                    [UVec4::ZERO; Self::MAX_UNIFORM_ITEMS];
                *cluster_offsets_and_counts.get_mut().data = [UVec4::ZERO; Self::MAX_UNIFORM_ITEMS];
            }
            ViewClusterBuffers::Storage {
                clusterable_object_index_lists,
                cluster_offsets_and_counts,
                ..
            } => {
                clusterable_object_index_lists.get_mut().data.clear();
                cluster_offsets_and_counts.get_mut().data.clear();
            }
        }
    }

    fn push_offset_and_counts(&mut self, offset: usize, counts: &ClusterableObjectCounts) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_offsets_and_counts,
                ..
            } => {
                let array_index = self.n_offsets >> 2; // >> 2 is equivalent to / 4
                if array_index >= Self::MAX_UNIFORM_ITEMS {
                    warn!("cluster offset and count out of bounds!");
                    return;
                }
                let component = self.n_offsets & ((1 << 2) - 1);
                let packed =
                    pack_offset_and_counts(offset, counts.point_lights, counts.spot_lights);

                cluster_offsets_and_counts.get_mut().data[array_index][component] = packed;
            }
            ViewClusterBuffers::Storage {
                cluster_offsets_and_counts,
                ..
            } => {
                cluster_offsets_and_counts.get_mut().data.push([
                    uvec4(
                        offset as u32,
                        counts.point_lights,
                        counts.spot_lights,
                        counts.reflection_probes,
                    ),
                    uvec4(counts.irradiance_volumes, counts.decals, 0, 0),
                ]);
            }
        }

        self.n_offsets += 1;
    }

    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    pub fn push_index(&mut self, index: usize) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                clusterable_object_index_lists,
                ..
            } => {
                let array_index = self.n_indices >> 4; // >> 4 is equivalent to / 16
                let component = (self.n_indices >> 2) & ((1 << 2) - 1);
                let sub_index = self.n_indices & ((1 << 2) - 1);
                let index = index as u32;

                clusterable_object_index_lists.get_mut().data[array_index][component] |=
                    index << (8 * sub_index);
            }
            ViewClusterBuffers::Storage {
                clusterable_object_index_lists,
                ..
            } => {
                clusterable_object_index_lists
                    .get_mut()
                    .data
                    .push(index as u32);
            }
        }

        self.n_indices += 1;
    }

    pub fn write_buffers(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                clusterable_object_index_lists,
                cluster_offsets_and_counts,
            } => {
                clusterable_object_index_lists.write_buffer(render_device, render_queue);
                cluster_offsets_and_counts.write_buffer(render_device, render_queue);
            }
            ViewClusterBuffers::Storage {
                clusterable_object_index_lists,
                cluster_offsets_and_counts,
            } => {
                clusterable_object_index_lists.write_buffer(render_device, render_queue);
                cluster_offsets_and_counts.write_buffer(render_device, render_queue);
            }
        }
    }

    pub fn clusterable_object_index_lists_binding(&self) -> Option<BindingResource> {
        match &self.buffers {
            ViewClusterBuffers::Uniform {
                clusterable_object_index_lists,
                ..
            } => clusterable_object_index_lists.binding(),
            ViewClusterBuffers::Storage {
                clusterable_object_index_lists,
                ..
            } => clusterable_object_index_lists.binding(),
        }
    }

    pub fn offsets_and_counts_binding(&self) -> Option<BindingResource> {
        match &self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_offsets_and_counts,
                ..
            } => cluster_offsets_and_counts.binding(),
            ViewClusterBuffers::Storage {
                cluster_offsets_and_counts,
                ..
            } => cluster_offsets_and_counts.binding(),
        }
    }

    pub fn min_size_clusterable_object_index_lists(
        buffer_binding_type: BufferBindingType,
    ) -> NonZero<u64> {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => GpuClusterableObjectIndexListsStorage::min_size(),
            BufferBindingType::Uniform => GpuClusterableObjectIndexListsUniform::min_size(),
        }
    }

    pub fn min_size_cluster_offsets_and_counts(
        buffer_binding_type: BufferBindingType,
    ) -> NonZero<u64> {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => GpuClusterOffsetsAndCountsStorage::min_size(),
            BufferBindingType::Uniform => GpuClusterOffsetsAndCountsUniform::min_size(),
        }
    }
}

impl ViewClusterBuffers {
    fn new(buffer_binding_type: BufferBindingType) -> Self {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => Self::storage(),
            BufferBindingType::Uniform => Self::uniform(),
        }
    }

    fn uniform() -> Self {
        ViewClusterBuffers::Uniform {
            clusterable_object_index_lists: UniformBuffer::default(),
            cluster_offsets_and_counts: UniformBuffer::default(),
        }
    }

    fn storage() -> Self {
        ViewClusterBuffers::Storage {
            clusterable_object_index_lists: StorageBuffer::default(),
            cluster_offsets_and_counts: StorageBuffer::default(),
        }
    }
}

// Compresses the offset and counts of point and spot lights so that they fit in
// a UBO.
//
// This function is only used if storage buffers are unavailable on this
// platform: typically, on WebGL 2.
//
// NOTE: With uniform buffer max binding size as 16384 bytes
// that means we can fit 204 clusterable objects in one uniform
// buffer, which means the count can be at most 204 so it
// needs 9 bits.
// The array of indices can also use u8 and that means the
// offset in to the array of indices needs to be able to address
// 16384 values. log2(16384) = 14 bits.
// We use 32 bits to store the offset and counts so
// we pack the offset into the upper 14 bits of a u32,
// the point light count into bits 9-17, and the spot light count into bits 0-8.
//  [ 31     ..     18 | 17      ..      9 | 8       ..     0 ]
//  [      offset      | point light count | spot light count ]
//
// NOTE: This assumes CPU and GPU endianness are the same which is true
// for all common and tested x86/ARM CPUs and AMD/NVIDIA/Intel/Apple/etc GPUs
//
// NOTE: On platforms that use this function, we don't cluster light probes, so
// the number of light probes is irrelevant.
fn pack_offset_and_counts(offset: usize, point_count: u32, spot_count: u32) -> u32 {
    ((offset as u32 & CLUSTER_OFFSET_MASK) << (CLUSTER_COUNT_SIZE * 2))
        | ((point_count & CLUSTER_COUNT_MASK) << CLUSTER_COUNT_SIZE)
        | (spot_count & CLUSTER_COUNT_MASK)
}

#[derive(ShaderType)]
struct GpuClusterableObjectIndexListsUniform {
    data: Box<[UVec4; ViewClusterBindings::MAX_UNIFORM_ITEMS]>,
}

// NOTE: Assert at compile time that GpuClusterableObjectIndexListsUniform
// fits within the maximum uniform buffer binding size
const _: () = assert!(GpuClusterableObjectIndexListsUniform::SHADER_SIZE.get() <= 16384);

impl Default for GpuClusterableObjectIndexListsUniform {
    fn default() -> Self {
        Self {
            data: Box::new([UVec4::ZERO; ViewClusterBindings::MAX_UNIFORM_ITEMS]),
        }
    }
}

impl Default for GpuClusterOffsetsAndCountsUniform {
    fn default() -> Self {
        Self {
            data: Box::new([UVec4::ZERO; ViewClusterBindings::MAX_UNIFORM_ITEMS]),
        }
    }
}
