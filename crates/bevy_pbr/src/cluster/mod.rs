//! Spatial clustering of objects, currently just point and spot lights.

use std::num::NonZeroU64;

use bevy_core_pipeline::core_3d::Camera3d;
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    query::{With, Without},
    reflect::ReflectComponent,
    system::{Commands, Query, Res, Resource},
    world::{FromWorld, World},
};
use bevy_math::{AspectRatio, UVec2, UVec3, UVec4, Vec3Swizzles as _, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::Camera,
    render_resource::{
        BindingResource, BufferBindingType, ShaderSize as _, ShaderType, StorageBuffer,
        UniformBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
    Extract,
};
use bevy_utils::{hashbrown::HashSet, tracing::warn};

pub(crate) use crate::cluster::assign::assign_objects_to_clusters;
use crate::MeshPipeline;

mod assign;

#[cfg(test)]
mod test;

// NOTE: this must be kept in sync with the same constants in pbr.frag
pub const MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS: usize = 256;

// NOTE: Clustered-forward rendering requires 3 storage buffer bindings so check that
// at least that many are supported using this constant and SupportedBindingType::from_device()
pub const CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT: u32 = 3;

// this must match CLUSTER_COUNT_SIZE in pbr.wgsl
// and must be large enough to contain MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS
const CLUSTER_COUNT_SIZE: u32 = 9;

const CLUSTER_OFFSET_MASK: u32 = (1 << (32 - (CLUSTER_COUNT_SIZE * 2))) - 1;
const CLUSTER_COUNT_MASK: u32 = (1 << CLUSTER_COUNT_SIZE) - 1;

// Clustered-forward rendering notes
// The main initial reference material used was this rather accessible article:
// http://www.aortiz.me/2018/12/21/CG.html
// Some inspiration was taken from “Practical Clustered Shading” which is part 2 of:
// https://efficientshading.com/2015/01/01/real-time-many-light-management-and-shadows-with-clustered-shading/
// (Also note that Part 3 of the above shows how we could support the shadow mapping for many lights.)
// The z-slicing method mentioned in the aortiz article is originally from Tiago Sousa's Siggraph 2016 talk about Doom 2016:
// http://advances.realtimerendering.com/s2016/Siggraph2016_idTech6.pdf

/// Configure the far z-plane mode used for the furthest depth slice for clustered forward
/// rendering
#[derive(Debug, Copy, Clone, Reflect)]
pub enum ClusterFarZMode {
    /// Calculate the required maximum z-depth based on currently visible
    /// clusterable objects.  Makes better use of available clusters, speeding
    /// up GPU lighting operations at the expense of some CPU time and using
    /// more indices in the clusterable object index lists.
    MaxClusterableObjectRange,
    /// Constant max z-depth
    Constant(f32),
}

/// Configure the depth-slicing strategy for clustered forward rendering
#[derive(Debug, Copy, Clone, Reflect)]
#[reflect(Default)]
pub struct ClusterZConfig {
    /// Far `Z` plane of the first depth slice
    pub first_slice_depth: f32,
    /// Strategy for how to evaluate the far `Z` plane of the furthest depth slice
    pub far_z_mode: ClusterFarZMode,
}

/// Configuration of the clustering strategy for clustered forward rendering
#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub enum ClusterConfig {
    /// Disable cluster calculations for this view
    None,
    /// One single cluster. Optimal for low-light complexity scenes or scenes where
    /// most lights affect the entire scene.
    Single,
    /// Explicit `X`, `Y` and `Z` counts (may yield non-square `X/Y` clusters depending on the aspect ratio)
    XYZ {
        dimensions: UVec3,
        z_config: ClusterZConfig,
        /// Specify if clusters should automatically resize in `X/Y` if there is a risk of exceeding
        /// the available cluster-object index limit
        dynamic_resizing: bool,
    },
    /// Fixed number of `Z` slices, `X` and `Y` calculated to give square clusters
    /// with at most total clusters. For top-down games where lights will generally always be within a
    /// short depth range, it may be useful to use this configuration with 1 or few `Z` slices. This
    /// would reduce the number of lights per cluster by distributing more clusters in screen space
    /// `X/Y` which matches how lights are distributed in the scene.
    FixedZ {
        total: u32,
        z_slices: u32,
        z_config: ClusterZConfig,
        /// Specify if clusters should automatically resize in `X/Y` if there is a risk of exceeding
        /// the available clusterable object index limit
        dynamic_resizing: bool,
    },
}

#[derive(Component, Debug, Default)]
pub struct Clusters {
    /// Tile size
    pub(crate) tile_size: UVec2,
    /// Number of clusters in `X` / `Y` / `Z` in the view frustum
    pub(crate) dimensions: UVec3,
    /// Distance to the far plane of the first depth slice. The first depth slice is special
    /// and explicitly-configured to avoid having unnecessarily many slices close to the camera.
    pub(crate) near: f32,
    pub(crate) far: f32,
    pub(crate) clusterable_objects: Vec<VisibleClusterableObjects>,
}

#[derive(Clone, Component, Debug, Default)]
pub struct VisibleClusterableObjects {
    pub(crate) entities: Vec<Entity>,
    pub point_light_count: usize,
    pub spot_light_count: usize,
}

#[derive(Resource, Default)]
pub struct GlobalVisibleClusterableObjects {
    pub(crate) entities: HashSet<Entity>,
}

#[derive(Resource)]
pub struct GlobalClusterableObjectMeta {
    pub gpu_clusterable_objects: GpuClusterableObjects,
    pub entity_to_index: EntityHashMap<usize>,
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
    ClusterHeader(u32, u32),
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
    #[size(runtime)]
    data: Vec<UVec4>,
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

impl Default for ClusterZConfig {
    fn default() -> Self {
        Self {
            first_slice_depth: 5.0,
            far_z_mode: ClusterFarZMode::MaxClusterableObjectRange,
        }
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        // 24 depth slices, square clusters with at most 4096 total clusters
        // use max light distance as clusters max `Z`-depth, first slice extends to 5.0
        Self::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: ClusterZConfig::default(),
            dynamic_resizing: true,
        }
    }
}

impl ClusterConfig {
    fn dimensions_for_screen_size(&self, screen_size: UVec2) -> UVec3 {
        match &self {
            ClusterConfig::None => UVec3::ZERO,
            ClusterConfig::Single => UVec3::ONE,
            ClusterConfig::XYZ { dimensions, .. } => *dimensions,
            ClusterConfig::FixedZ {
                total, z_slices, ..
            } => {
                let aspect_ratio: f32 =
                    AspectRatio::from_pixels(screen_size.x, screen_size.y).into();
                let mut z_slices = *z_slices;
                if *total < z_slices {
                    warn!("ClusterConfig has more z-slices than total clusters!");
                    z_slices = *total;
                }
                let per_layer = *total as f32 / z_slices as f32;

                let y = f32::sqrt(per_layer / aspect_ratio);

                let mut x = (y * aspect_ratio) as u32;
                let mut y = y as u32;

                // check extremes
                if x == 0 {
                    x = 1;
                    y = per_layer as u32;
                }
                if y == 0 {
                    x = per_layer as u32;
                    y = 1;
                }

                UVec3::new(x, y, z_slices)
            }
        }
    }

    fn first_slice_depth(&self) -> f32 {
        match self {
            ClusterConfig::None | ClusterConfig::Single => 0.0,
            ClusterConfig::XYZ { z_config, .. } | ClusterConfig::FixedZ { z_config, .. } => {
                z_config.first_slice_depth
            }
        }
    }

    fn far_z_mode(&self) -> ClusterFarZMode {
        match self {
            ClusterConfig::None => ClusterFarZMode::Constant(0.0),
            ClusterConfig::Single => ClusterFarZMode::MaxClusterableObjectRange,
            ClusterConfig::XYZ { z_config, .. } | ClusterConfig::FixedZ { z_config, .. } => {
                z_config.far_z_mode
            }
        }
    }

    fn dynamic_resizing(&self) -> bool {
        match self {
            ClusterConfig::None | ClusterConfig::Single => false,
            ClusterConfig::XYZ {
                dynamic_resizing, ..
            }
            | ClusterConfig::FixedZ {
                dynamic_resizing, ..
            } => *dynamic_resizing,
        }
    }
}

impl Clusters {
    fn update(&mut self, screen_size: UVec2, requested_dimensions: UVec3) {
        debug_assert!(
            requested_dimensions.x > 0 && requested_dimensions.y > 0 && requested_dimensions.z > 0
        );

        let tile_size = (screen_size.as_vec2() / requested_dimensions.xy().as_vec2())
            .ceil()
            .as_uvec2()
            .max(UVec2::ONE);
        self.tile_size = tile_size;
        self.dimensions = (screen_size.as_vec2() / tile_size.as_vec2())
            .ceil()
            .as_uvec2()
            .extend(requested_dimensions.z)
            .max(UVec3::ONE);

        // NOTE: Maximum 4096 clusters due to uniform buffer size constraints
        debug_assert!(self.dimensions.x * self.dimensions.y * self.dimensions.z <= 4096);
    }
    fn clear(&mut self) {
        self.tile_size = UVec2::ONE;
        self.dimensions = UVec3::ZERO;
        self.near = 0.0;
        self.far = 0.0;
        self.clusterable_objects.clear();
    }
}

pub fn add_clusters(
    mut commands: Commands,
    cameras: Query<(Entity, Option<&ClusterConfig>, &Camera), (Without<Clusters>, With<Camera3d>)>,
) {
    for (entity, config, camera) in &cameras {
        if !camera.is_active {
            continue;
        }

        let config = config.copied().unwrap_or_default();
        // actual settings here don't matter - they will be overwritten in
        // `assign_objects_to_clusters``
        commands
            .entity(entity)
            .insert((Clusters::default(), config));
    }
}

impl VisibleClusterableObjects {
    #[inline]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Entity> {
        self.entities.iter()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

impl GlobalVisibleClusterableObjects {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }
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

    pub fn min_size(buffer_binding_type: BufferBindingType) -> NonZeroU64 {
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

#[allow(clippy::too_many_arguments)]
// Sort clusterable objects by:
//
// * point-light vs spot-light, so that we can iterate point lights and spot
//   lights in contiguous blocks in the fragment shader,
//
// * then those with shadows enabled first, so that the index can be used to
//   render at most `point_light_shadow_maps_count` point light shadows and
//   `spot_light_shadow_maps_count` spot light shadow maps,
//
// * then by entity as a stable key to ensure that a consistent set of
//   clusterable objects are chosen if the clusterable object count limit is
//   exceeded.
pub(crate) fn clusterable_object_order(
    (entity_1, shadows_enabled_1, is_spot_light_1): (&Entity, &bool, &bool),
    (entity_2, shadows_enabled_2, is_spot_light_2): (&Entity, &bool, &bool),
) -> std::cmp::Ordering {
    is_spot_light_1
        .cmp(is_spot_light_2) // pointlights before spot lights
        .then_with(|| shadows_enabled_2.cmp(shadows_enabled_1)) // shadow casters before non-casters
        .then_with(|| entity_1.cmp(entity_2)) // stable
}

/// Extracts clusters from the main world from the render world.
pub fn extract_clusters(
    mut commands: Commands,
    views: Extract<Query<(Entity, &Clusters, &Camera)>>,
) {
    for (entity, clusters, camera) in &views {
        if !camera.is_active {
            continue;
        }

        let num_entities: usize = clusters
            .clusterable_objects
            .iter()
            .map(|l| l.entities.len())
            .sum();
        let mut data = Vec::with_capacity(clusters.clusterable_objects.len() + num_entities);
        for cluster_objects in &clusters.clusterable_objects {
            data.push(ExtractedClusterableObjectElement::ClusterHeader(
                cluster_objects.point_light_count as u32,
                cluster_objects.spot_light_count as u32,
            ));
            for clusterable_entity in &cluster_objects.entities {
                data.push(ExtractedClusterableObjectElement::ClusterableObjectEntity(
                    *clusterable_entity,
                ));
            }
        }

        commands.get_or_spawn(entity).insert((
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
                ExtractedClusterableObjectElement::ClusterHeader(
                    point_light_count,
                    spot_light_count,
                ) => {
                    let offset = view_clusters_bindings.n_indices();
                    view_clusters_bindings.push_offset_and_counts(
                        offset,
                        *point_light_count as usize,
                        *spot_light_count as usize,
                    );
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

        commands.get_or_spawn(entity).insert(view_clusters_bindings);
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

    pub fn push_offset_and_counts(&mut self, offset: usize, point_count: usize, spot_count: usize) {
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
                let packed = pack_offset_and_counts(offset, point_count, spot_count);

                cluster_offsets_and_counts.get_mut().data[array_index][component] = packed;
            }
            ViewClusterBuffers::Storage {
                cluster_offsets_and_counts,
                ..
            } => {
                cluster_offsets_and_counts.get_mut().data.push(UVec4::new(
                    offset as u32,
                    point_count as u32,
                    spot_count as u32,
                    0,
                ));
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
    ) -> NonZeroU64 {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => GpuClusterableObjectIndexListsStorage::min_size(),
            BufferBindingType::Uniform => GpuClusterableObjectIndexListsUniform::min_size(),
        }
    }

    pub fn min_size_cluster_offsets_and_counts(
        buffer_binding_type: BufferBindingType,
    ) -> NonZeroU64 {
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

// NOTE: With uniform buffer max binding size as 16384 bytes
// that means we can fit 256 clusterable objects in one uniform
// buffer, which means the count can be at most 256 so it
// needs 9 bits.
// The array of indices can also use u8 and that means the
// offset in to the array of indices needs to be able to address
// 16384 values. log2(16384) = 14 bits.
// We use 32 bits to store the offset and counts so
// we pack the offset into the upper 14 bits of a u32,
// the point light count into bits 9-17, and the spot light count into bits 0-8.
//  [ 31     ..     18 | 17      ..      9 | 8       ..     0 ]
//  [      offset      | point light count | spot light count ]
// NOTE: This assumes CPU and GPU endianness are the same which is true
// for all common and tested x86/ARM CPUs and AMD/NVIDIA/Intel/Apple/etc GPUs
fn pack_offset_and_counts(offset: usize, point_count: usize, spot_count: usize) -> u32 {
    ((offset as u32 & CLUSTER_OFFSET_MASK) << (CLUSTER_COUNT_SIZE * 2))
        | (point_count as u32 & CLUSTER_COUNT_MASK) << CLUSTER_COUNT_SIZE
        | (spot_count as u32 & CLUSTER_COUNT_MASK)
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
