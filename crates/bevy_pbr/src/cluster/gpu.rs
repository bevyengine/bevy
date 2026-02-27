//! Clustering of lights and other clusterable objects on GPU.
//!
//! GPU light clustering uses the hardware rasterizer for compute purposes as a
//! way to automatically distribute workloads within 2D axis-aligned bounding
//! boxes without actually rendering any pixels. The algorithm is as follows,
//! with each step corresponding to a raster or compute command
//!
//! 1. *Z slicing*: We have a 3D cluster froxel grid of size W×H×D and seek to
//!    rasterize D axis-aligned quads, each of size W×H, representing the range of
//!    each clusterable object. In this compute phase, we generate D indirect
//!    instances for each clusterable object for the subsequent indirect draws.
//!
//! 2. *Count rasterization*: We use instanced indirect drawing to rasterize
//!    each quad generated in step 1 to a viewport of size W×H, with color
//!    writes disabled. Each rasterized fragment represents a cluster-object
//!    pair. In the fragment shader, we check to see if the object
//!    intersects the cluster, and, if it does, we atomically bump a counter
//!    corresponding to the number of objects of the given type intersecting
//!    the cluster in question. We don't record the ID of the object in this
//!    phase; we simply count the number of objects.
//!
//! 3. *Local allocation*: Now that we know the number of objects of each
//!    type in each cluster, we can proceed to allocate space in the
//!    clustered object buffer for each clustered object list. To do this,
//!    we need to perform a [*prefix sum*] operation so that each list is
//!    tightly packed with the others. For example, if adjacent clusters
//!    have 2, 5, and 3 objects, they'll be allocated at offsets 0, 2, and 7
//!    respectively. This *local* step uses a [Hillis-Steele scan] in shared
//!    memory to compute the prefix sum of each chunk of 256 clusters. We
//!    can't go beyond 256 clusters in this local step because 256 is the
//!    maximum workgroup size in `wgpu`.
//!
//! 4. *Global allocation*: To deal with the fact that we can't calculate
//!    prefix sums beyond 256 clusters in step 3, we employ this second step
//!    that does a sequential loop over every 256-cluster chunk, propagating
//!    the prefix sum. At the end of this step, every list of clustered
//!    objects is allocated.
//!
//! 5. *Populate rasterization*: Finally, we issue an instanced indirect
//!    draw command using the same parameters as step (2). We test each
//!    cluster-object pair for intersection, and, if the test passes, we
//!    record the ID of each clustered object into the correct space in the
//!    list, using an scratch pad buffer of atomics to store the position of
//!    the next object in each list.
//!
//! [*prefix sum*]: https://en.wikipedia.org/wiki/Prefix_sum
//!
//! [Hillis-Steele scan]: https://en.wikipedia.org/wiki/Prefix_sum#Algorithm_1:_Shorter_span,_more_parallel

use alloc::sync::Arc;
use std::sync::Mutex;

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::Camera;
use bevy_color::Color;
use bevy_core_pipeline::{prepass::node::early_prepass, Core3d, Core3dSystems};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_light::{
    cluster::{Clusters, GlobalClusterGpuSettings, GlobalClusterSettings},
    EnvironmentMapLight, IrradianceVolume,
};
use bevy_material::descriptor::{
    BindGroupLayoutDescriptor, CachedComputePipelineId, CachedRenderPipelineId,
    ComputePipelineDescriptor, FragmentState, RenderPipelineDescriptor, VertexState,
};
use bevy_math::{vec2, Vec2};
use bevy_mesh::{VertexBufferLayout, VertexFormat};
use bevy_render::{
    diagnostic::RecordDiagnostics as _,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::{
        binding_types,
        encase::internal::{CreateFrom as _, Reader},
        BindGroup, BindGroupEntry, BindGroupLayoutEntries, Buffer, BufferBindingType,
        BufferDescriptor, BufferInitDescriptor, BufferUsages, ColorTargetState, ColorWrites,
        CommandEncoder, ComputePassDescriptor, ComputePipeline, Extent3d, IndexFormat, LoadOp,
        MapMode, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipeline, ShaderStages, ShaderType, SpecializedComputePipeline,
        SpecializedComputePipelines, SpecializedRenderPipeline, SpecializedRenderPipelines,
        StorageBuffer, StoreOp, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        UninitBufferVec, VertexAttribute, VertexStepMode,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
    sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet, RenderEntity},
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, ViewUniform, ViewUniformOffset, ViewUniforms},
    MainWorld, Render, RenderApp, RenderSystems,
};
use bevy_shader::{load_shader_library, Shader, ShaderDefVal};
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};
use tracing::{error, trace, warn};

use crate::{
    cluster::{
        GpuClusterOffsetAndCounts, GpuClusterOffsetsAndCountsStorage,
        GpuClusterableObjectIndexListsStorage, ViewClusterBuffers,
    },
    decal::clustered::{DecalsBuffer, RenderClusteredDecal, RenderClusteredDecals},
    gpu_clustering_is_enabled, ExtractedClusterConfig, GlobalClusterableObjectMeta,
    GpuClusteredLight, GpuLights, LightMeta, LightProbesBuffer, LightProbesUniform,
    RenderViewLightProbes, ViewClusterBindings, ViewLightProbesUniformOffset,
    ViewLightsUniformOffset,
};

/// The workgroup size of the `cluster_allocate.wgsl` shader.
const ALLOCATION_WORKGROUP_SIZE: u32 = 256;
/// The workgroup size of the `cluster_z_slice.wgsl` shader.
const Z_SLICING_WORKGROUP_SIZE: u32 = 64;

/// A plugin that enables GPU clustering of lights and other objects.
pub struct GpuClusteringPlugin;

impl Plugin for GpuClusteringPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "cluster.wgsl");
        embedded_asset!(app, "cluster_z_slice.wgsl");
        embedded_asset!(app, "cluster_raster.wgsl");
        embedded_asset!(app, "cluster_allocate.wgsl");

        app.add_plugins(ExtractResourcePlugin::<
            GlobalClusterSettings,
            GpuClusteringPlugin,
        >::default());
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // Bail out if we have no storage buffers. This is the case when we have
        // `WGPU_SETTINGS_PRIO="webgl2"`.
        let render_device = render_app.world().resource::<RenderDevice>();
        if render_device.limits().max_storage_buffers_per_shader_stage == 0 {
            return;
        }

        render_app
            .init_resource::<SpecializedRenderPipelines<ClusteringRasterPipeline>>()
            .init_resource::<SpecializedComputePipelines<ClusteringZSlicingPipeline>>()
            .init_resource::<SpecializedComputePipelines<ClusteringAllocationPipeline>>()
            .init_resource::<RenderViewClusteringReadbackData>()
            .init_resource::<GpuClusteringMeshBuffers>()
            .init_resource::<ClusteringRasterPipeline>()
            .init_resource::<ClusteringZSlicingPipeline>()
            .init_resource::<ClusteringAllocationPipeline>()
            .add_systems(
                Render,
                (prepare_clustering_pipelines, prepare_cluster_dummy_textures)
                    .in_set(RenderSystems::Prepare)
                    .run_if(gpu_clustering_is_enabled),
            )
            .add_systems(
                Render,
                (
                    prepare_clusters_for_gpu_clustering,
                    upload_view_gpu_clustering_buffers,
                )
                    .chain()
                    .in_set(RenderSystems::PrepareResources)
                    .run_if(gpu_clustering_is_enabled),
            )
            .add_systems(
                Render,
                prepare_clustering_bind_groups
                    .in_set(RenderSystems::PrepareBindGroups)
                    .run_if(gpu_clustering_is_enabled),
            )
            .add_systems(
                Core3d,
                cluster_on_gpu
                    .before(early_prepass)
                    .in_set(Core3dSystems::Prepass)
                    .run_if(gpu_clustering_is_enabled),
            );
    }
}

/// The texture that we bind when performing the raster passes.
///
/// We don't actually write to this texture; it exists only so that we can set a
/// viewport.
#[derive(Component, Deref, DerefMut)]
pub struct ViewClusteringDummyTexture(CachedTexture);

/// The bind groups for each pass of GPU clustering.
#[derive(Component)]
pub struct ViewClusteringBindGroups {
    /// The bind group for the Z-slicing compute pass.
    clustering_bind_group_z_slicing_pass: BindGroup,
    /// The bind group for the count rasterization pass.
    clustering_bind_group_count_pass: BindGroup,
    /// The bind group for both local and global allocation passes.
    clustering_bind_group_allocate_pass: BindGroup,
    /// The bind group for the populate rasterization pass.
    clustering_bind_group_populate_pass: BindGroup,
}

/// The GPU representation of a single Z-slice of a clusterable object.
///
/// A Z-slice is an axis-aligned bounding box representing the potential
/// bounding box of a clusterable object in a single Z slice of the froxel grid.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ShaderType, Pod, Zeroable)]
#[repr(C)]
pub struct ClusterableObjectZSlice {
    /// The index of the object to be clustered.
    pub object_index: u32,
    /// The type of the object to be clustered.
    ///
    /// This is one of the `CLUSTERABLE_OBJECT_TYPE_` constants in
    /// `cluster.wgsl`.
    pub object_type: u32,
    /// The Z coordinate of the froxels that this slice covers.
    pub z_slice: u32,
}

/// Metadata stored on GPU that's global to all clusters for a view.
#[derive(Clone, Copy, Default, ShaderType, Pod, Zeroable)]
#[repr(C)]
pub struct ClusterMetadata {
    /// The indirect draw parameters for the raster passes.
    indirect_draw_params: ClusterRasterIndirectDrawParams,

    /// The total number of clustered lights, set by the CPU.
    clustered_light_count: u32,
    /// The total number of reflection probes, set by the CPU.
    reflection_probe_count: u32,
    /// The total number of irradiance volumes, set by the CPU.
    irradiance_volume_count: u32,
    /// The total number of clustered decals, set by the CPU.
    decal_count: u32,

    /// The current maximum size of the Z-slice list.
    z_slice_list_capacity: u32,

    /// The current size of the clustered object index list.
    ///
    /// This is set to 0 by the CPU, and the GPU updates it with the computed
    /// value.
    index_list_capacity: u32,

    /// The farthest depth that any clustered object AABB has extended to this
    /// frame.
    ///
    /// This is set to 0 by the CPU, and the GPU updates it with the computed
    /// value.
    farthest_z: f32,
}

/// Indirect draw parameters for the raster dispatch phase, built partially by
/// the CPU and partially by the GPU.
///
/// These must conform to the format that `wgpu` demands, so this structure
/// layout must not be modified.
#[derive(Clone, Copy, Default, ShaderType, Pod, Zeroable)]
#[repr(C)]
pub struct ClusterRasterIndirectDrawParams {
    index_count: u32,

    /// Represents the total number of Z slices.
    ///
    /// This field is the one that the GPU modifies.
    instance_count: u32,

    first_index: u32,
    base_vertex: u32,
    first_instance: u32,
}

/// A component, stored on [`ExtractedView`], that stores buffers needed to
/// perform GPU clustering for that view.
#[derive(Component)]
pub struct ViewGpuClusteringBuffers {
    /// The buffer that holds the Z slices for each clusterable object.
    ///
    /// The `cluster_z_slice.wgsl` shader fills this buffer out, and the raster
    /// passes read it.
    pub z_slices_buffer: UninitBufferVec<ClusterableObjectZSlice>,
    /// The buffer that holds the scratchpad offsets and counts for each
    /// clusterable object.
    ///
    /// The populate pass uses this to coordinate where to write indices for
    /// each clusterable object. The allocation pass zeroes it out.
    scratchpad_offsets_and_counts_buffer: UninitBufferVec<GpuClusterOffsetAndCounts>,
    /// The buffer that stores the [`ClusterMetadata`].
    ///
    /// Since this buffer is small, [`StorageBuffer`] is fine to use.
    cluster_metadata_buffer: StorageBuffer<ClusterMetadata>,
}

impl ViewGpuClusteringBuffers {
    /// Creates a new, empty set of [`ViewGpuClusteringBuffers`] for a single
    /// view.
    pub(crate) fn new() -> ViewGpuClusteringBuffers {
        let mut cluster_metadata_buffer = StorageBuffer::from(ClusterMetadata::default());
        cluster_metadata_buffer.add_usages(BufferUsages::COPY_SRC | BufferUsages::INDIRECT);
        cluster_metadata_buffer.set_label(Some("clustering Z slicing metadata buffer"));

        ViewGpuClusteringBuffers {
            cluster_metadata_buffer,
            z_slices_buffer: UninitBufferVec::new(BufferUsages::STORAGE | BufferUsages::COPY_DST),
            scratchpad_offsets_and_counts_buffer: UninitBufferVec::new(
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            ),
        }
    }
}

/// Stores data associated with reading back clustering statistics from GPU to
/// CPU for all views.
#[derive(Resource, Default)]
pub(crate) struct RenderViewClusteringReadbackData {
    /// The data for each view.
    ///
    /// This is locked behind a mutex so that the buffer readback callbacks,
    /// which execute concurrently, can access it alongside the render world.
    views: MainEntityHashMap<Arc<Mutex<ViewClusteringReadbackData>>>,
}

/// Data associated with reading back clustering statistics for a single view.
struct ViewClusteringReadbackData {
    /// The current capacity of the Z slice list.
    ///
    /// This starts out at the default size as specified by the allocation and
    /// can grow based on the results of GPU readback.
    z_slice_list_capacity: usize,
    /// The current capacity of the clustered object index list.
    ///
    /// This starts out at the default size as specified by the allocation and
    /// can grow based on the results of GPU readback.
    max_index_list_capacity: usize,
    /// Buffers corresponding to GPU readback operations in progress.
    metadata_staging_pending_buffers: Vec<Buffer>,
    /// Buffers corresponding to GPU readback operations that are finished.
    ///
    /// These buffers are ready for reuse.
    metadata_staging_free_buffers: Vec<Buffer>,
    /// Statistics about GPU clustering that the GPU calculated last frame.
    last_frame_statistics: Option<ViewClusteringLastFrameStatistics>,
}

/// Statistics about GPU clustering that the GPU calculated last frame.
struct ViewClusteringLastFrameStatistics {
    /// The actual used size of the index list.
    ///
    /// If this is greater than the capacity of the index list, the CPU will
    /// resize the index list buffer.
    index_list_size: u32,
    /// The maximum depth of all axis-aligned bounding boxes corresponding to
    /// clusterable objects in view.
    farthest_z: f32,
}

impl ViewClusteringReadbackData {
    /// Creates a new [`ViewClusteringReadbackData`] for a view.
    ///
    /// The [`Self::z_slice_list_capacity`] and
    /// [`Self::max_index_list_capacity`] are calculated based on the initial
    /// capacities that the application set in the [`GlobalClusterGpuSettings`].
    fn new(settings: &GlobalClusterGpuSettings) -> ViewClusteringReadbackData {
        ViewClusteringReadbackData {
            z_slice_list_capacity: settings.initial_z_slice_list_capacity,
            max_index_list_capacity: settings.initial_index_list_capacity,
            metadata_staging_pending_buffers: vec![],
            metadata_staging_free_buffers: vec![],
            last_frame_statistics: None,
        }
    }

    fn get_or_create_staging_buffer(&mut self, render_device: &RenderDevice) -> Buffer {
        let staging_buffer = self.metadata_staging_free_buffers.pop().unwrap_or_else(|| {
            render_device.create_buffer(&BufferDescriptor {
                label: Some("clustering metadata staging buffer"),
                size: ClusterMetadata::min_size().into(),
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        });
        self.metadata_staging_pending_buffers
            .push(staging_buffer.clone());
        staging_buffer
    }

    /// Updates this [`ViewClusteringReadbackData`] with new information from
    /// the given metadata read back from the GPU.
    fn update_from_metadata(&mut self, gpu_clustering_metadata: &ClusterMetadata) {
        // Schedule a resize of the Z slice list if the GPU overflowed.
        if self.z_slice_list_capacity
            < gpu_clustering_metadata.indirect_draw_params.instance_count as usize
        {
            let new_capacity = gpu_clustering_metadata
                .indirect_draw_params
                .instance_count
                .next_power_of_two();
            warn!(
                "Resizing the view clustering Z slice list from a capacity of {0} elements to \
                a capacity of {1} elements. The scene lighting may have been corrupted for a \
                few frames. To avoid this, set the `gpu_clustering.z_slice_list_capacity` field \
                on the `GlobalClusterSettings` resource to at least {1}.",
                self.z_slice_list_capacity, new_capacity
            );
            self.z_slice_list_capacity = new_capacity as usize;
        }

        // Schedule a resize of the index slice list if the GPU overflowed.
        if self.max_index_list_capacity < gpu_clustering_metadata.index_list_capacity as usize {
            let new_capacity = gpu_clustering_metadata
                .index_list_capacity
                .next_power_of_two();
            warn!(
                "Resizing the view clustering index list from a capacity of {0} elements to a \
                capacity of {1} elements. The scene lighting may have been corrupted for a \
                few frames. To avoid this, set the `gpu_clustering.index_list_capacity` field on \
                the `GlobalClusterSettings` resource to at least {1}.",
                self.max_index_list_capacity, new_capacity
            );
            self.max_index_list_capacity = new_capacity as usize;
        }

        // Record the statistics we just received.
        self.last_frame_statistics = Some(ViewClusteringLastFrameStatistics {
            index_list_size: gpu_clustering_metadata.index_list_capacity,
            farthest_z: gpu_clustering_metadata.farthest_z,
        });
    }
}

/// Global data relating to the `cluster_raster.wgsl` shader.
#[derive(Resource)]
pub struct ClusteringRasterPipeline {
    /// The bind group layout for group 0 for the count (first) pass.
    pub bind_group_layout_count_pass: BindGroupLayoutDescriptor,
    /// The bind group layout for group 0 for the populate (second) pass.
    pub bind_group_layout_populate_pass: BindGroupLayoutDescriptor,
    /// A handle to the shader itself.
    pub shader: Handle<Shader>,
}

/// Global data relating to the `cluster_z_slice.wgsl` shader.
#[derive(Resource)]
pub struct ClusteringZSlicingPipeline {
    /// The bind group layout for group 0.
    pub bind_group_layout: BindGroupLayoutDescriptor,
    /// A handle to the shader itself.
    pub shader: Handle<Shader>,
}

/// Global data relating to the `cluster_allocate.wgsl` shader.
#[derive(Resource)]
pub struct ClusteringAllocationPipeline {
    /// The bind group layout of group 0 for both shader invocations.
    pub bind_group_layout: BindGroupLayoutDescriptor,
    /// A handle to the `cluster_allocate.wgsl` shader itself.
    pub shader: Handle<Shader>,
}

/// The pipeline key that identifies specializations of the
/// `cluster_raster.wgsl` shader.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClusteringRasterPipelineKey {
    /// True if this is the populate (second) pass; false if it's the count
    /// (first) one.
    populate_pass: bool,
}

/// The pipeline key that identifies specializations of the
/// `cluster_allocate.wgsl` shader.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClusteringAllocationPipelineKey {
    /// True if this is the global (second) pass; false if it's the local
    /// (first) one.
    global_pass: bool,
}

impl FromWorld for ClusteringRasterPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let mut bind_group_layout_entries_count_pass = vec![
            // @group(0) @binding(0) var<storage> z_slices:
            // array<ClusterableObjectZSlice>;
            binding_types::storage_buffer_read_only::<ClusterableObjectZSlice>(false)
                .build(0, ShaderStages::VERTEX_FRAGMENT),
            // @group(0) @binding(1) var<storage, read_write> index_lists:
            // ClusterableObjectIndexLists;
            binding_types::storage_buffer::<GpuClusterableObjectIndexListsStorage>(false)
                .build(1, ShaderStages::VERTEX_FRAGMENT),
            // @group(0) @binding(2) var<storage> clustered_lights:
            // ClusteredLights;
            binding_types::storage_buffer_read_only::<GpuClusteredLight>(false)
                .build(2, ShaderStages::VERTEX_FRAGMENT),
            // @group(0) @binding(3) var<uniform> light_probes: LightProbes;
            binding_types::uniform_buffer::<LightProbesUniform>(true)
                .build(3, ShaderStages::VERTEX_FRAGMENT),
            // @group(0) @binding(4) var<storage> clustered_decals:
            // ClusteredDecals;
            binding_types::storage_buffer_read_only::<RenderClusteredDecal>(false)
                .build(4, ShaderStages::VERTEX_FRAGMENT),
            // @group(0) @binding(5) var<uniform> lights: Lights;
            binding_types::uniform_buffer::<GpuLights>(true)
                .build(5, ShaderStages::VERTEX_FRAGMENT),
            // @group(0) @binding(6) var<uniform> view: View;
            binding_types::uniform_buffer::<ViewUniform>(true)
                .build(6, ShaderStages::VERTEX_FRAGMENT),
        ];

        let mut bind_group_layout_entries_populate_pass =
            bind_group_layout_entries_count_pass.clone();

        // @group(0) @binding(7) var<storage, read_write> offsets_and_counts:
        // ClusterOffsetsAndCountsAtomic;
        bind_group_layout_entries_count_pass.push(
            binding_types::storage_buffer::<GpuClusterOffsetsAndCountsStorage>(false)
                .build(7, ShaderStages::VERTEX_FRAGMENT),
        );

        // @group(0) @binding(7) var<storage> offsets_and_counts:
        // ClusterOffsetsAndCounts;
        bind_group_layout_entries_populate_pass.push(
            binding_types::storage_buffer_read_only::<GpuClusterOffsetsAndCountsStorage>(false)
                .build(7, ShaderStages::VERTEX_FRAGMENT),
        );
        // @group(0) @binding(8) var<storage, read_write>
        // scratchpad_offsets_and_counts: ClusterOffsetsAndCountsAtomic;
        bind_group_layout_entries_populate_pass.push(
            binding_types::storage_buffer::<GpuClusterOffsetsAndCountsStorage>(false)
                .build(8, ShaderStages::VERTEX_FRAGMENT),
        );

        let bind_group_layout_count_pass = BindGroupLayoutDescriptor::new(
            "clustering count pass bind group layout",
            &bind_group_layout_entries_count_pass,
        );
        let bind_group_layout_populate_pass = BindGroupLayoutDescriptor::new(
            "clustering populate pass bind group layout",
            &bind_group_layout_entries_populate_pass,
        );

        let shader = load_embedded_asset!(asset_server, "cluster_raster.wgsl");

        ClusteringRasterPipeline {
            bind_group_layout_count_pass,
            bind_group_layout_populate_pass,
            shader,
        }
    }
}

impl SpecializedRenderPipeline for ClusteringRasterPipeline {
    type Key = ClusteringRasterPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];
        if key.populate_pass {
            shader_defs.push(ShaderDefVal::from("POPULATE_PASS"));
        } else {
            shader_defs.push(ShaderDefVal::from("COUNT_PASS"));
        }

        RenderPipelineDescriptor {
            label: if key.populate_pass {
                Some("clustering populate pipeline".into())
            } else {
                Some("clustering count pipeline".into())
            },
            layout: vec![if key.populate_pass {
                self.bind_group_layout_populate_pass.clone()
            } else {
                self.bind_group_layout_count_pass.clone()
            }],
            immediate_size: 0,
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some("vertex_main".into()),
                buffers: vec![VertexBufferLayout {
                    array_stride: size_of::<Vec2>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some("fragment_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::R8Unorm,
                    blend: None,
                    // Disable writing.
                    write_mask: ColorWrites::empty(),
                })],
            }),
            ..default()
        }
    }
}

impl FromWorld for ClusteringZSlicingPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let bind_group_layout = BindGroupLayoutDescriptor::new(
            "clustering Z slicing pass bind group layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // @group(0) @binding(0) var<storage, read_write>
                    // cluster_metadata: ClusterMetadata;
                    binding_types::storage_buffer::<ClusterMetadata>(false),
                    // @group(0) @binding(1) var<storage, read_write> z_slices:
                    // array<ClusterableObjectZSlice>;
                    binding_types::storage_buffer::<ClusterableObjectZSlice>(false),
                    // @group(0) @binding(2) var<storage> clustered_lights:
                    // ClusteredLights;
                    binding_types::storage_buffer_read_only::<GpuClusteredLight>(false),
                    // @group(0) @binding(3) var<uniform> light_probes:
                    // LightProbes;
                    binding_types::uniform_buffer::<LightProbesUniform>(true),
                    // @group(0) @binding(4) var<storage> clustered_decals:
                    // ClusteredDecals;
                    binding_types::storage_buffer_read_only::<RenderClusteredDecal>(false),
                    // @group(0) @binding(5) var<uniform> lights: Lights;
                    binding_types::uniform_buffer::<GpuLights>(true),
                    // @group(0) @binding(6) var<uniform> view: View;
                    binding_types::uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        let shader = load_embedded_asset!(asset_server, "cluster_z_slice.wgsl");

        ClusteringZSlicingPipeline {
            bind_group_layout,
            shader,
        }
    }
}

impl SpecializedComputePipeline for ClusteringZSlicingPipeline {
    type Key = ();

    fn specialize(&self, _: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("clustering Z slicing pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            shader: self.shader.clone(),
            shader_defs: vec![],
            entry_point: Some("z_slice_main".into()),
            zero_initialize_workgroup_memory: true,
            ..default()
        }
    }
}

impl FromWorld for ClusteringAllocationPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let bind_group_layout = BindGroupLayoutDescriptor::new(
            "clustering allocation pass bind group layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // @group(0) @binding(0) var<storage, read_write>
                    // offsets_and_counts: ClusterOffsetsAndCounts;
                    binding_types::storage_buffer::<GpuClusterOffsetsAndCountsStorage>(false),
                    // @group(0) @binding(1) var<uniform> lights: Lights;
                    binding_types::uniform_buffer::<GpuLights>(true),
                    // @group(0) @binding(2) var<storage, read_write>
                    // clustering_metadata: ClusterMetadata;
                    binding_types::storage_buffer::<ClusterMetadata>(false),
                    // @group(0) @binding(3) var<storage, read_write>
                    // scratchpad_offsets_and_counts: ClusterOffsetsAndCounts;
                    binding_types::storage_buffer::<GpuClusterOffsetsAndCountsStorage>(false),
                ),
            ),
        );

        let shader = load_embedded_asset!(asset_server, "cluster_allocate.wgsl");

        ClusteringAllocationPipeline {
            bind_group_layout,
            shader,
        }
    }
}

impl SpecializedComputePipeline for ClusteringAllocationPipeline {
    type Key = ClusteringAllocationPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: if key.global_pass {
                Some("clustering allocation global pass pipeline".into())
            } else {
                Some("clustering allocation local pass pipeline".into())
            },
            layout: vec![self.bind_group_layout.clone()],
            shader: self.shader.clone(),
            shader_defs: vec![],
            entry_point: if key.global_pass {
                Some("allocate_global_main".into())
            } else {
                Some("allocate_local_main".into())
            },
            zero_initialize_workgroup_memory: true,
            ..default()
        }
    }
}

/// The vertices of the quad that we rasterize to represent a clusterable object
/// Z slice.
static GPU_CLUSTERING_VERTICES: [Vec2; 4] = [
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 1.0),
];

/// The indices of the quad that we rasterize to represent a clusterable object
/// Z slice.
static GPU_CLUSTERING_INDICES: [u32; 6] = [0, 1, 2, 1, 3, 2];

/// The buffers that store the vertices and indices for the quad that we
/// rasterize to represent each clusterable object Z slice.
#[derive(Resource)]
struct GpuClusteringMeshBuffers {
    /// The vertex buffer containing the 4 vertices of a quad.
    vertex_buffer: Buffer,
    /// The index buffer containing the 6 indices of a quad.
    index_buffer: Buffer,
}

impl FromWorld for GpuClusteringMeshBuffers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        GpuClusteringMeshBuffers {
            vertex_buffer: render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("GPU clustering vertex buffer"),
                contents: bytemuck::bytes_of(&GPU_CLUSTERING_VERTICES),
                usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
            }),
            index_buffer: render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("GPU clustering index buffer"),
                contents: bytemuck::bytes_of(&GPU_CLUSTERING_INDICES),
                usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
            }),
        }
    }
}

/// The IDs of each pipeline used for GPU clustering for a single view.
#[derive(Component)]
pub struct ViewGpuClusteringPipelineIds {
    /// The compute pipeline for the Z slicing compute pass (pass 1).
    clustering_z_slicing_pipeline_id: CachedComputePipelineId,
    /// The compute pipeline for the count raster pass (pass 2).
    clustering_count_pipeline_id: CachedRenderPipelineId,
    /// The compute pipeline for the local allocation compute pass (pass 3).
    clustering_allocation_local_pipeline_id: CachedComputePipelineId,
    /// The compute pipeline for the global allocation compute pass (pass 4).
    clustering_allocation_global_pipeline_id: CachedComputePipelineId,
    /// The compute pipeline for the populate raster pass (pass 5).
    clustering_populate_pipeline_id: CachedRenderPipelineId,
}

/// The render command building system that performs GPU clustering on each
/// view.
fn cluster_on_gpu(
    view_query: ViewQuery<(
        &MainEntity,
        Option<&ViewGpuClusteringBuffers>,
        Option<&ViewGpuClusteringPipelineIds>,
        Option<&ViewClusteringDummyTexture>,
        Option<&ViewClusteringBindGroups>,
        Option<&ViewLightProbesUniformOffset>,
        Option<&ViewLightsUniformOffset>,
        Option<&ViewUniformOffset>,
        Option<&ExtractedClusterConfig>,
    )>,
    pipeline_cache: Res<PipelineCache>,
    clustering_mesh_buffers: Res<GpuClusteringMeshBuffers>,
    render_view_clustering_readback_data: Res<RenderViewClusteringReadbackData>,
    mut render_context: RenderContext,
) {
    let (
        view_main_entity,
        Some(view_gpu_clustering_buffers),
        Some(view_gpu_clustering_pipeline_ids),
        Some(view_clustering_dummy_texture),
        Some(view_clustering_bind_groups),
        Some(view_light_probes_uniform_offset),
        Some(view_lights_uniform_offset),
        Some(view_uniform_offset),
        Some(extracted_cluster_config),
    ) = view_query.into_inner()
    else {
        trace!("Failed to match view query; not clustering");
        return;
    };

    let Some(view_clustering_readback_data) = render_view_clustering_readback_data
        .views
        .get(view_main_entity)
    else {
        return;
    };

    let (
        Some(clustering_z_slicing_compute_pipeline),
        Some(clustering_count_render_pipeline),
        Some(clustering_allocate_local_compute_pipeline),
        Some(clustering_allocate_global_compute_pipeline),
        Some(clustering_populate_render_pipeline),
    ) = (
        pipeline_cache.get_compute_pipeline(
            view_gpu_clustering_pipeline_ids.clustering_z_slicing_pipeline_id,
        ),
        pipeline_cache
            .get_render_pipeline(view_gpu_clustering_pipeline_ids.clustering_count_pipeline_id),
        pipeline_cache.get_compute_pipeline(
            view_gpu_clustering_pipeline_ids.clustering_allocation_local_pipeline_id,
        ),
        pipeline_cache.get_compute_pipeline(
            view_gpu_clustering_pipeline_ids.clustering_allocation_global_pipeline_id,
        ),
        pipeline_cache
            .get_render_pipeline(view_gpu_clustering_pipeline_ids.clustering_populate_pipeline_id),
    )
    else {
        trace!("One or more clustering pipelines not found; not clustering");
        return;
    };

    let diagnostics = render_context.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(render_context.command_encoder(), "clustering");

    // Fetch a staging buffer for us to perform readback with.
    let staging_buffer = view_clustering_readback_data
        .lock()
        .unwrap()
        .get_or_create_staging_buffer(render_context.render_device());

    let command_encoder = render_context.command_encoder();
    command_encoder.push_debug_group("clustering");

    // Pass 1: Z slicing.
    run_clustering_z_slicing_pass(
        command_encoder,
        clustering_z_slicing_compute_pipeline,
        &view_clustering_bind_groups.clustering_bind_group_z_slicing_pass,
        &view_gpu_clustering_buffers.cluster_metadata_buffer,
        view_light_probes_uniform_offset,
        view_lights_uniform_offset,
        view_uniform_offset,
    );

    // Pass 2: Count raster.
    run_clustering_rasterization_pass(
        command_encoder,
        clustering_count_render_pipeline,
        &view_clustering_bind_groups.clustering_bind_group_count_pass,
        view_gpu_clustering_buffers,
        view_light_probes_uniform_offset,
        view_lights_uniform_offset,
        view_uniform_offset,
        view_clustering_dummy_texture,
        extracted_cluster_config,
        &clustering_mesh_buffers,
        false,
    );

    // Pass 3: local allocation.
    run_clustering_allocation_pass(
        command_encoder,
        clustering_allocate_local_compute_pipeline,
        view_clustering_bind_groups,
        view_lights_uniform_offset,
        extracted_cluster_config,
        false,
    );

    // Pass 4: global allocation.
    run_clustering_allocation_pass(
        command_encoder,
        clustering_allocate_global_compute_pipeline,
        view_clustering_bind_groups,
        view_lights_uniform_offset,
        extracted_cluster_config,
        true,
    );

    // Pass 5: populate raster.
    run_clustering_rasterization_pass(
        command_encoder,
        clustering_populate_render_pipeline,
        &view_clustering_bind_groups.clustering_bind_group_populate_pass,
        view_gpu_clustering_buffers,
        view_light_probes_uniform_offset,
        view_lights_uniform_offset,
        view_uniform_offset,
        view_clustering_dummy_texture,
        extracted_cluster_config,
        &clustering_mesh_buffers,
        true,
    );

    // Schedule a readback of the readback data.
    schedule_readback_staging(
        command_encoder,
        view_gpu_clustering_buffers,
        &staging_buffer,
    );
    schedule_readback_buffer_map(
        command_encoder,
        view_clustering_readback_data.clone(),
        &staging_buffer,
    );

    command_encoder.pop_debug_group();
    time_span.end(render_context.command_encoder());

    /// Runs the Z slicing pass (step 1).
    fn run_clustering_z_slicing_pass(
        command_encoder: &mut CommandEncoder,
        clustering_z_slicing_pipeline: &ComputePipeline,
        clustering_z_slicing_bind_group: &BindGroup,
        clustering_cluster_metadata_buffer: &StorageBuffer<ClusterMetadata>,
        view_light_probes_uniform_offset: &ViewLightProbesUniformOffset,
        view_lights_uniform_offset: &ViewLightsUniformOffset,
        view_uniform_offset: &ViewUniformOffset,
    ) {
        let mut compute_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("clustering Z slicing pass"),
            ..default()
        });
        compute_pass.set_pipeline(clustering_z_slicing_pipeline);
        compute_pass.set_bind_group(
            0,
            Some(&**clustering_z_slicing_bind_group),
            &[
                **view_light_probes_uniform_offset,
                view_lights_uniform_offset.offset,
                view_uniform_offset.offset,
            ],
        );

        let clustering_cluster_metadata = clustering_cluster_metadata_buffer.get();
        let clusterable_object_count = clustering_cluster_metadata.clustered_light_count
            + clustering_cluster_metadata.reflection_probe_count
            + clustering_cluster_metadata.irradiance_volume_count
            + clustering_cluster_metadata.decal_count;

        let workgroup_count = clusterable_object_count.div_ceil(Z_SLICING_WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    /// Runs either the count or populate rasterization pass (steps 2 and 5
    /// respectively) for a single view.
    ///
    /// The `populate_pass` parameter specifies whether this is a count pass
    /// (false) or a populate pass (true).
    fn run_clustering_rasterization_pass(
        command_encoder: &mut CommandEncoder,
        clustering_render_pipeline: &RenderPipeline,
        clustering_bind_group: &BindGroup,
        view_gpu_clustering_buffers: &ViewGpuClusteringBuffers,
        view_light_probes_uniform_offset: &ViewLightProbesUniformOffset,
        view_lights_uniform_offset: &ViewLightsUniformOffset,
        view_uniform_offset: &ViewUniformOffset,
        view_clustering_dummy_texture: &ViewClusteringDummyTexture,
        extracted_cluster_config: &ExtractedClusterConfig,
        clustering_mesh_buffers: &GpuClusteringMeshBuffers,
        populate_pass: bool,
    ) {
        let Some(cluster_metadata_buffer) =
            view_gpu_clustering_buffers.cluster_metadata_buffer.buffer()
        else {
            error!("Z slicing metadata buffer was never uploaded");
            return;
        };

        let mut render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: if populate_pass {
                Some("clustering populate pass")
            } else {
                Some("clustering count pass")
            },
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view_clustering_dummy_texture.default_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    // Do nothing to the color buffer. We only care about using
                    // the rasterizer for fragment scheduling; we're not going
                    // to actually paint any pixels.
                    load: LoadOp::Clear(Color::BLACK.to_linear().into()),
                    store: StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: None,
            ..default()
        });
        render_pass.set_pipeline(clustering_render_pipeline);
        render_pass.set_bind_group(
            0,
            Some(&**clustering_bind_group),
            &[
                **view_light_probes_uniform_offset,
                view_lights_uniform_offset.offset,
                view_uniform_offset.offset,
            ],
        );

        // Since we rounded up the dummy texture size to prevent thrashing, we
        // need to use an explicit viewport here so that we only render to the
        // correct portion.
        render_pass.set_viewport(
            0.0,
            0.0,
            extracted_cluster_config.dimensions.x as f32,
            extracted_cluster_config.dimensions.y as f32,
            0.0,
            1.0,
        );

        render_pass.set_vertex_buffer(0, *clustering_mesh_buffers.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            *clustering_mesh_buffers.index_buffer.slice(..),
            IndexFormat::Uint32,
        );
        render_pass.draw_indexed_indirect(cluster_metadata_buffer, 0);
    }

    /// Runs either the local or global allocation pass (steps 3 and 4
    /// respectively) for GPU clustering for a single view.
    ///
    /// The `global_pass` parameter specifies whether this is the local pass
    /// (false) or the global pass (true).
    fn run_clustering_allocation_pass(
        command_encoder: &mut CommandEncoder,
        clustering_allocation_pipeline: &ComputePipeline,
        view_clustering_bind_groups: &ViewClusteringBindGroups,
        view_lights_uniform_offset: &ViewLightsUniformOffset,
        extracted_cluster_config: &ExtractedClusterConfig,
        global_pass: bool,
    ) {
        let mut compute_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: if global_pass {
                Some("clustering allocation global pass")
            } else {
                Some("clustering allocation local pass")
            },
            ..default()
        });
        compute_pass.set_pipeline(clustering_allocation_pipeline);
        compute_pass.set_bind_group(
            0,
            Some(&*view_clustering_bind_groups.clustering_bind_group_allocate_pass),
            &[view_lights_uniform_offset.offset],
        );

        // The global pass has only one workgroup because it runs sequentially
        // over chunks, while the local pass has a number of workgroups equal to
        // the number of chunks because it runs in parallel over them.
        let workgroup_count = if global_pass {
            1
        } else {
            extracted_cluster_config
                .dimensions
                .element_product()
                .div_ceil(ALLOCATION_WORKGROUP_SIZE)
        };
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    /// Schedules the staging part of readback of the data from GPU.
    fn schedule_readback_staging(
        command_encoder: &mut CommandEncoder,
        view_gpu_clustering_buffers: &ViewGpuClusteringBuffers,
        staging_buffer: &Buffer,
    ) {
        match view_gpu_clustering_buffers.cluster_metadata_buffer.buffer() {
            None => {
                // This should never happen. It shouldn't have been possible to
                // create the necessary bind groups without this buffer's being
                // present.
                error!("No clustering Z slicing metadata buffer found");
            }
            Some(metadata_buffer) => {
                // Copy the metadata buffer to the staging buffer so we can read
                // it back.
                command_encoder.copy_buffer_to_buffer(
                    metadata_buffer,
                    0,
                    staging_buffer,
                    0,
                    Some(u64::from(ClusterMetadata::min_size())),
                );
            }
        }
    }

    /// Schedules the buffer map operation part of the readback of the data from
    /// GPU.
    fn schedule_readback_buffer_map(
        command_encoder: &mut CommandEncoder,
        view_clustering_readback_data: Arc<Mutex<ViewClusteringReadbackData>>,
        staging_buffer: &Buffer,
    ) {
        let captured_staging_buffer = staging_buffer.clone();
        command_encoder.map_buffer_on_submit(staging_buffer, MapMode::Read, .., move |result| {
            if result.is_err() {
                return;
            };

            let mut view_clustering_readback_data = view_clustering_readback_data.lock().unwrap();

            {
                // Use `encase` to populate a `ClusterMetadata`.
                let buffer_view = captured_staging_buffer.slice(..).get_mapped_range();
                let Ok(mut buffer_reader) =
                    Reader::new::<ClusterMetadata>(buffer_view[..].to_vec(), 0)
                else {
                    return;
                };
                let gpu_clustering_metadata = ClusterMetadata::create_from(&mut buffer_reader);

                // Update readback data.
                view_clustering_readback_data.update_from_metadata(&gpu_clustering_metadata);
            }

            // `wgpu` will error if we didn't drop the buffer view at this
            // point, which is why we use a separate block above.
            captured_staging_buffer.unmap();

            // Recycle the staging buffer.
            view_clustering_readback_data
                .metadata_staging_free_buffers
                .push(captured_staging_buffer);
        });
    }
}

/// Prepares bind groups for each of the shaders involved in GPU clustering.
fn prepare_clustering_bind_groups(
    mut commands: Commands,
    views_query: Query<
        (Entity, &ViewGpuClusteringBuffers, &ViewClusterBindings),
        With<ExtractedView>,
    >,
    render_device: Res<RenderDevice>,
    clustering_z_slicing_pipeline: Res<ClusteringZSlicingPipeline>,
    clustering_raster_pipeline: Res<ClusteringRasterPipeline>,
    clustering_allocation_pipeline: Res<ClusteringAllocationPipeline>,
    global_clusterable_object_meta: Res<GlobalClusterableObjectMeta>,
    pipeline_cache: Res<PipelineCache>,
    light_probes_buffer: Res<LightProbesBuffer>,
    decals_buffer: Res<DecalsBuffer>,
    light_meta: Res<LightMeta>,
    view_uniforms: Res<ViewUniforms>,
) {
    let (
        Some(gpu_clustered_lights_binding),
        Some(light_probes_binding),
        Some(decals_buffer),
        Some(lights_binding),
        Some(view_binding),
    ) = (
        global_clusterable_object_meta
            .gpu_clustered_lights
            .binding(),
        light_probes_buffer.binding(),
        decals_buffer.buffer(),
        light_meta.view_gpu_lights.binding(),
        view_uniforms.uniforms.binding(),
    )
    else {
        return;
    };

    // Create separate bind groups for each view.
    for (view_entity, view_gpu_clustering_buffers, view_cluster_bindings) in &views_query {
        let ViewClusterBuffers::Storage {
            clusterable_object_index_lists: ref maybe_clusterable_object_index_lists,
            cluster_offsets_and_counts: ref maybe_cluster_offsets_and_counts,
        } = view_cluster_bindings.buffers
        else {
            continue;
        };

        let (
            Some(z_slices_buffer),
            Some(cluster_metadata_buffer),
            Some(scratchpad_offsets_and_counts_buffer),
            Some(clusterable_object_index_lists),
            Some(cluster_offsets_and_counts),
        ) = (
            view_gpu_clustering_buffers.z_slices_buffer.buffer(),
            view_gpu_clustering_buffers.cluster_metadata_buffer.buffer(),
            view_gpu_clustering_buffers
                .scratchpad_offsets_and_counts_buffer
                .buffer(),
            maybe_clusterable_object_index_lists.buffer(),
            maybe_cluster_offsets_and_counts.buffer(),
        )
        else {
            continue;
        };

        let clustering_bind_group_entries_z_slicing_pass = [
            // @group(0) @binding(0) var<storage, read_write>
            // cluster_metadata: ClusterMetadata;
            BindGroupEntry {
                binding: 0,
                resource: cluster_metadata_buffer.as_entire_binding(),
            },
            // @group(0) @binding(1) var<storage, read_write> z_slices:
            // array<ClusterableObjectZSlice>;
            BindGroupEntry {
                binding: 1,
                resource: z_slices_buffer.as_entire_binding(),
            },
            // @group(0) @binding(2) var<storage> clustered_lights:
            // ClusteredLights;
            BindGroupEntry {
                binding: 2,
                resource: gpu_clustered_lights_binding.clone(),
            },
            // @group(0) @binding(3) var<uniform> light_probes: LightProbes;
            BindGroupEntry {
                binding: 3,
                resource: light_probes_binding.clone(),
            },
            // @group(0) @binding(4) var<storage> clustered_decals:
            // ClusteredDecals;
            BindGroupEntry {
                binding: 4,
                resource: decals_buffer.as_entire_binding(),
            },
            // @group(0) @binding(5) var<uniform> lights: Lights;
            BindGroupEntry {
                binding: 5,
                resource: lights_binding.clone(),
            },
            // @group(0) @binding(6) var<uniform> view: View;
            BindGroupEntry {
                binding: 6,
                resource: view_binding.clone(),
            },
        ];

        let mut clustering_bind_group_entries_count_pass: Vec<BindGroupEntry> = vec![
            // @group(0) @binding(0) var<storage> z_slices:
            // array<ClusterableObjectZSlice>;
            BindGroupEntry {
                binding: 0,
                resource: z_slices_buffer.as_entire_binding(),
            },
            // @group(0) @binding(1) var<storage, read_write> index_lists:
            // ClusterableObjectIndexLists;
            BindGroupEntry {
                binding: 1,
                resource: clusterable_object_index_lists.as_entire_binding(),
            },
            // @group(0) @binding(2) var<storage> clustered_lights:
            // ClusteredLights;
            BindGroupEntry {
                binding: 2,
                resource: gpu_clustered_lights_binding.clone(),
            },
            // @group(0) @binding(3) var<uniform> light_probes: LightProbes;
            BindGroupEntry {
                binding: 3,
                resource: light_probes_binding.clone(),
            },
            // @group(0) @binding(4) var<storage> clustered_decals:
            // ClusteredDecals;
            BindGroupEntry {
                binding: 4,
                resource: decals_buffer.as_entire_binding(),
            },
            // @group(0) @binding(5) var<uniform> lights: Lights;
            BindGroupEntry {
                binding: 5,
                resource: lights_binding.clone(),
            },
            // @group(0) @binding(6) var<uniform> view: View;
            BindGroupEntry {
                binding: 6,
                resource: view_binding.clone(),
            },
        ];

        let mut clustering_bind_group_entries_populate_pass =
            clustering_bind_group_entries_count_pass.clone();

        clustering_bind_group_entries_count_pass.push(
            // @group(0) @binding(7) var<storage, read_write>
            // offsets_and_counts: ClusterOffsetsAndCounts;
            BindGroupEntry {
                binding: 7,
                resource: cluster_offsets_and_counts.as_entire_binding(),
            },
        );

        clustering_bind_group_entries_populate_pass.push(
            // @group(0) @binding(7) var<storage>
            // offsets_and_counts: ClusterOffsetsAndCounts;
            BindGroupEntry {
                binding: 7,
                resource: cluster_offsets_and_counts.as_entire_binding(),
            },
        );
        clustering_bind_group_entries_populate_pass.push(
            // @group(0) @binding(8) var<storage, read_write>
            // scratchpad_offsets_and_counts: ClusterOffsetsAndCountsAtomic;
            BindGroupEntry {
                binding: 8,
                resource: scratchpad_offsets_and_counts_buffer.as_entire_binding(),
            },
        );

        let clustering_bind_group_entries_allocation_pass: [BindGroupEntry; _] = [
            // @group(0) @binding(0) var<storage, read_write>
            // offsets_and_counts: ClusterOffsetsAndCounts;
            BindGroupEntry {
                binding: 0,
                resource: cluster_offsets_and_counts.as_entire_binding(),
            },
            // @group(0) @binding(1) var<uniform> lights: Lights;
            BindGroupEntry {
                binding: 1,
                resource: lights_binding.clone(),
            },
            // @group(0) @binding(2) var<storage, read_write>
            // clustering_metadata: ClusterMetadata;
            BindGroupEntry {
                binding: 2,
                resource: cluster_metadata_buffer.as_entire_binding(),
            },
            // @group(0) @binding(3) var<storage, read_write>
            // scratchpad_offsets_and_counts: ClusterOffsetsAndCounts;
            BindGroupEntry {
                binding: 3,
                resource: scratchpad_offsets_and_counts_buffer.as_entire_binding(),
            },
        ];

        let clustering_bind_group_z_slicing_pass = render_device.create_bind_group(
            "clustering Z slicing pass bind group",
            &pipeline_cache.get_bind_group_layout(&clustering_z_slicing_pipeline.bind_group_layout),
            &clustering_bind_group_entries_z_slicing_pass,
        );
        let clustering_bind_group_count_pass = render_device.create_bind_group(
            "clustering count pass bind group",
            &pipeline_cache
                .get_bind_group_layout(&clustering_raster_pipeline.bind_group_layout_count_pass),
            &clustering_bind_group_entries_count_pass,
        );
        let clustering_bind_group_allocate_pass = render_device.create_bind_group(
            "clustering allocate pass bind group",
            &pipeline_cache
                .get_bind_group_layout(&clustering_allocation_pipeline.bind_group_layout),
            &clustering_bind_group_entries_allocation_pass,
        );
        let clustering_bind_group_populate_pass = render_device.create_bind_group(
            "clustering populate pass bind group",
            &pipeline_cache
                .get_bind_group_layout(&clustering_raster_pipeline.bind_group_layout_populate_pass),
            &clustering_bind_group_entries_populate_pass,
        );

        commands
            .entity(view_entity)
            .insert(ViewClusteringBindGroups {
                clustering_bind_group_z_slicing_pass,
                clustering_bind_group_count_pass,
                clustering_bind_group_allocate_pass,
                clustering_bind_group_populate_pass,
            });
    }
}

/// Creates the dummy textures that we use to establish a viewport for the
/// rasterization phases of GPU clustering.
///
/// We don't actually write to these textures, but they need to exist so that a
/// viewport of the appropriate size can be set.
fn prepare_cluster_dummy_textures(
    mut commands: Commands,
    views_query: Query<(Entity, &ExtractedClusterConfig), With<ExtractedView>>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
) {
    for (view_entity, view_cluster_config) in &views_query {
        let dummy_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("clustering dummy texture"),
                // We round these up to the nearest multiple of 32 to guard
                // against the risk of thrashing between different sizes,
                // especially if the auto-resize feature is on.
                size: Extent3d {
                    width: round_up(view_cluster_config.dimensions.x),
                    height: round_up(view_cluster_config.dimensions.y),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
                view_formats: &[],
            },
        );
        commands
            .entity(view_entity)
            .insert(ViewClusteringDummyTexture(dummy_texture));
    }

    /// Rounds the given value up to the nearest multiple of 32.
    fn round_up(length: u32) -> u32 {
        (length + 31) & !31
    }
}

/// Prepares the compute and raster pipelines for the various shader invocations
/// in GPU clustering for each view.
fn prepare_clustering_pipelines(
    mut commands: Commands,
    views_query: Query<Entity, With<ExtractedView>>,
    pipeline_cache: Res<PipelineCache>,
    mut clustering_z_slicing_pipelines: ResMut<
        SpecializedComputePipelines<ClusteringZSlicingPipeline>,
    >,
    mut clustering_raster_pipelines: ResMut<SpecializedRenderPipelines<ClusteringRasterPipeline>>,
    mut clustering_allocation_pipelines: ResMut<
        SpecializedComputePipelines<ClusteringAllocationPipeline>,
    >,
    clustering_z_slicing_pipeline: Res<ClusteringZSlicingPipeline>,
    clustering_raster_pipeline: Res<ClusteringRasterPipeline>,
    clustering_allocation_pipeline: Res<ClusteringAllocationPipeline>,
) {
    for view_entity in &views_query {
        let clustering_z_slicing_pipeline_id = clustering_z_slicing_pipelines.specialize(
            &pipeline_cache,
            &clustering_z_slicing_pipeline,
            (),
        );
        let clustering_count_pipeline_id = clustering_raster_pipelines.specialize(
            &pipeline_cache,
            &clustering_raster_pipeline,
            ClusteringRasterPipelineKey {
                populate_pass: false,
            },
        );
        let clustering_local_allocation_pipeline_id = clustering_allocation_pipelines.specialize(
            &pipeline_cache,
            &clustering_allocation_pipeline,
            ClusteringAllocationPipelineKey { global_pass: false },
        );
        let clustering_global_allocation_pipeline_id = clustering_allocation_pipelines.specialize(
            &pipeline_cache,
            &clustering_allocation_pipeline,
            ClusteringAllocationPipelineKey { global_pass: true },
        );
        let clustering_populate_pipeline_id = clustering_raster_pipelines.specialize(
            &pipeline_cache,
            &clustering_raster_pipeline,
            ClusteringRasterPipelineKey {
                populate_pass: true,
            },
        );

        commands
            .entity(view_entity)
            .insert(ViewGpuClusteringPipelineIds {
                clustering_z_slicing_pipeline_id,
                clustering_count_pipeline_id,
                clustering_allocation_local_pipeline_id: clustering_local_allocation_pipeline_id,
                clustering_allocation_global_pipeline_id: clustering_global_allocation_pipeline_id,
                clustering_populate_pipeline_id,
            });
    }
}

/// Uploads the buffers needed to perform GPU clustering to the GPU.
fn upload_view_gpu_clustering_buffers(
    mut views_query: Query<&mut ViewGpuClusteringBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for mut view_gpu_clustering_buffers in &mut views_query {
        view_gpu_clustering_buffers
            .z_slices_buffer
            .write_buffer(&render_device);

        view_gpu_clustering_buffers
            .cluster_metadata_buffer
            .write_buffer(&render_device, &render_queue);

        // Make sure the scratchpad buffer is nonempty, and upload it.
        if view_gpu_clustering_buffers
            .scratchpad_offsets_and_counts_buffer
            .is_empty()
        {
            view_gpu_clustering_buffers
                .scratchpad_offsets_and_counts_buffer
                .add();
        }
        view_gpu_clustering_buffers
            .scratchpad_offsets_and_counts_buffer
            .write_buffer(&render_device);
    }
}

/// Extracts information needed for GPU clustering from each view in the render
/// world, and synchronizes statistics back from the render world to the main
/// world if needed.
pub fn extract_clusters_for_gpu_clustering(
    mut commands: Commands,
    mut main_world: ResMut<MainWorld>,
    render_view_clustering_index_list_sizes: Res<RenderViewClusteringReadbackData>,
) {
    let mut views = main_world.query::<(Entity, RenderEntity, &mut Clusters, &Camera)>();

    for (main_view_entity, render_view_entity, mut clusters, camera) in
        views.iter_mut(&mut main_world)
    {
        let mut entity_commands = commands
            .get_entity(render_view_entity)
            .expect("Clusters entity wasn't synced.");
        if !camera.is_active {
            entity_commands.remove::<ExtractedClusterConfig>();
            continue;
        }

        entity_commands.insert(ExtractedClusterConfig::from(&*clusters));

        // Read back statistics from the render world to the main world if we
        // have some.
        // The clustering systems in the main world will pick them up and adjust
        // cluster settings if necessary.
        if let Some(view_clustering_buffer_size_data) = render_view_clustering_index_list_sizes
            .views
            .get(&MainEntity::from(main_view_entity))
        {
            let view_clustering_buffer_size_data = view_clustering_buffer_size_data.lock().unwrap();
            if let Some(last_frame_statistics) =
                &view_clustering_buffer_size_data.last_frame_statistics
            {
                clusters.last_frame_farthest_z = Some(last_frame_statistics.farthest_z);
                clusters.last_frame_total_cluster_index_count =
                    Some(last_frame_statistics.index_list_size as usize);
            }
        }
    }

    let global_cluster_settings = main_world.resource::<GlobalClusterSettings>();
    commands.insert_resource(global_cluster_settings.clone());
}

/// Creates associated buffers necessary to perform GPU clustering for all
/// views.
pub(crate) fn prepare_clusters_for_gpu_clustering(
    mut commands: Commands,
    views_query: Query<(
        Entity,
        &MainEntity,
        &ExtractedClusterConfig,
        Option<&RenderViewLightProbes<EnvironmentMapLight>>,
        Option<&RenderViewLightProbes<IrradianceVolume>>,
    )>,
    render_clustered_decals: Res<RenderClusteredDecals>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    global_clusterable_object_meta: Res<GlobalClusterableObjectMeta>,
    global_cluster_settings: Res<GlobalClusterSettings>,
    mut render_view_clustering_index_list_sizes: ResMut<RenderViewClusteringReadbackData>,
) {
    let render_device = render_device.into_inner();

    let Some(ref global_cluster_settings_gpu) = global_cluster_settings.gpu_clustering else {
        error!("`prepare_clusters_for_gpu_clustering() called when not GPU clustering");
        return;
    };

    let gpu_clustered_lights_storage = &global_clusterable_object_meta.gpu_clustered_lights;

    let mut all_view_main_entities = MainEntityHashSet::default();

    for (
        view_entity,
        view_main_entity,
        extracted_cluster_config,
        maybe_environment_maps,
        maybe_irradiance_volumes,
    ) in &views_query
    {
        // Allocate the cluster array.
        let mut view_clusters_bindings =
            ViewClusterBindings::new(BufferBindingType::Storage { read_only: false });
        view_clusters_bindings.clear();
        let cluster_count = extracted_cluster_config.dimensions.x as usize
            * extracted_cluster_config.dimensions.y as usize
            * extracted_cluster_config.dimensions.z as usize;
        view_clusters_bindings.reserve_clusters(cluster_count);

        all_view_main_entities.insert(*view_main_entity);

        // Create the readback data.
        let view_clustering_buffer_size_data = render_view_clustering_index_list_sizes
            .views
            .entry(*view_main_entity)
            .or_insert_with(|| {
                Arc::new(Mutex::new(ViewClusteringReadbackData::new(
                    global_cluster_settings_gpu,
                )))
            })
            .lock()
            .unwrap();

        let mut view_gpu_clustering_buffers = ViewGpuClusteringBuffers::new();

        // Count the number of each type of clusterable object that we have.
        let clustered_light_count = gpu_clustered_lights_storage.data.len() as u32;
        let reflection_probe_count = match maybe_environment_maps {
            Some(view_reflection_probes) => view_reflection_probes.len() as u32,
            None => 0,
        };
        let irradiance_volume_count = match maybe_irradiance_volumes {
            Some(view_irradiance_volumes) => view_irradiance_volumes.len() as u32,
            None => 0,
        };
        let decal_count = render_clustered_decals.len() as u32;

        // Initialize the metadata.
        *view_gpu_clustering_buffers
            .cluster_metadata_buffer
            .get_mut() = ClusterMetadata {
            indirect_draw_params: ClusterRasterIndirectDrawParams {
                index_count: 6,
                // This will be filled in by the GPU.
                instance_count: 0,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            },
            clustered_light_count,
            reflection_probe_count,
            irradiance_volume_count,
            decal_count,
            index_list_capacity: view_clustering_buffer_size_data.max_index_list_capacity as u32,
            z_slice_list_capacity: view_clustering_buffer_size_data.z_slice_list_capacity as u32,
            farthest_z: 0.0,
        };

        // Allocate Z slices.
        if view_gpu_clustering_buffers.z_slices_buffer.len()
            < view_clustering_buffer_size_data.z_slice_list_capacity
        {
            view_gpu_clustering_buffers.z_slices_buffer.add_multiple(
                view_clustering_buffer_size_data.z_slice_list_capacity
                    - view_gpu_clustering_buffers.z_slices_buffer.len(),
            );
        }

        // Make room for the appropriate number of indices.
        view_clusters_bindings
            .reserve_indices(view_clustering_buffer_size_data.max_index_list_capacity);
        view_clusters_bindings.write_buffers(render_device, &render_queue);

        // Allocate scratchpad offsets and counts.
        view_gpu_clustering_buffers
            .scratchpad_offsets_and_counts_buffer
            .add_multiple(cluster_count);

        commands
            .entity(view_entity)
            .insert((view_clusters_bindings, view_gpu_clustering_buffers));
    }

    // Clear out clustering allocations corresponding to views that don't exist
    // any longer.
    render_view_clustering_index_list_sizes
        .views
        .retain(|view_main_entity, _| all_view_main_entities.contains(view_main_entity));
}

impl ExtractResource<GpuClusteringPlugin> for GlobalClusterSettings {
    type Source = GlobalClusterSettings;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}
