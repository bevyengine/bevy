//! Performs skinning and morph target evaluation in a compute shader.

use core::{
    fmt::{self, Display, Formatter},
    mem,
    num::NonZero,
};

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::schedule;
use bevy_ecs::{
    component::Component,
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Local, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_material::descriptor::{
    BindGroupLayoutDescriptor, CachedComputePipelineId, ComputePipelineDescriptor,
};
use bevy_math::{Mat4, Vec3, Vec4};
use bevy_mesh::{morph::MorphAttributes, Mesh, MeshVertexAttributeId, MeshVertexBufferLayoutRef};
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_reflect::Reflect;
use bevy_render::{
    batching::gpu_preprocessing::BatchedInstanceBuffers,
    diagnostic::RecordDiagnostics as _,
    mesh::{
        allocator::{MeshAllocator, MeshSlabId},
        RenderMesh,
    },
    render_asset::RenderAssets,
    render_resource::{
        self,
        binding_types::{storage_buffer, storage_buffer_read_only},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, Buffer,
        BufferBinding, BufferDescriptor, BufferUsages, ComputePassDescriptor, PipelineCache,
        RawBufferVec, ShaderStages, ShaderType, SpecializedComputePipeline,
        SpecializedComputePipelines, UninitBufferVec, VertexAttribute,
    },
    renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue},
    sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet},
    Render, RenderApp, RenderSystems,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};
use nonmax::NonMaxU32;
use tracing::error;

use crate::{
    prepare_mesh_bind_groups, prepare_skins, GpuMorphDescriptor, MeshInputUniform, MeshUniform,
    MorphIndices, MorphUniforms, RenderMeshInstances, SkinUniforms,
};

/// The workgroup size for the skin caching shader.
///
/// This must match the value in `skin_cache.wgsl`.
const SKIN_CACHE_WORKGROUP_SIZE: u32 = 64;

/// Add this component to a mesh that has a skin and/or a morph target to
/// request that the deformation be performed in a compute shader instead of a
/// vertex shader.
///
/// This is a performance tradeoff. It can reduce the amount of computation that
/// the GPU must perform, because the skin and/or morph targets are evaluated
/// once per frame instead of once per rendering pass. However, it increases the
/// GPU memory usage and increases the GPU memory bandwidth requirements. The
/// memory usage requirements are proportional to the number of mesh
/// *instances*, not the number of meshes; thus, if you have 100 instances of a
/// single skinned mesh, the memory requirements are 100 times greater than they
/// would be if you only had a single copy of the mesh.
#[derive(Clone, Copy, Component, Debug, Reflect)]
#[reflect(Component)]
pub struct CacheSkin;

/// A plugin that performs skin caching for entities with [`CacheSkin`]
/// components.
pub struct SkinCachePlugin;

impl Plugin for SkinCachePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "skin_cache.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<CachedSkinEntities>()
            .init_resource::<CachedSkinBindGroups>()
            .init_resource::<SkinCachePipelineIds>()
            .init_resource::<SpecializedComputePipelines<SkinCachePipeline>>()
            .add_systems(
                Render,
                prepare_skin_cache_buffers
                    .in_set(RenderSystems::PrepareMeshes)
                    .before(crate::collect_meshes_for_gpu_building),
            )
            .add_systems(
                Render,
                write_skin_cache_buffers.in_set(RenderSystems::PrepareResourcesFlush),
            )
            .add_systems(
                Render,
                prepare_skin_cache_bind_groups
                    .in_set(RenderSystems::PrepareBindGroups)
                    .after(prepare_mesh_bind_groups),
            )
            .add_systems(
                Render,
                prepare_skin_cache_pipelines
                    .in_set(RenderSystems::Prepare)
                    .after(prepare_skins),
            )
            .add_systems(
                RenderGraph,
                skin_cache
                    .before(schedule::camera_driver)
                    .after(render_resource::update_sparse_buffers),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SkinCachePipeline>()
            .init_resource::<CachedSkinBuffers>();
    }
}

/// A render graph system that invokes the skin caching shader in order to skin
/// and morph all mesh instances that have [`CacheSkin`] components.
fn skin_cache(
    pipeline_cache: Res<PipelineCache>,
    cached_skin_bind_groups: Res<CachedSkinBindGroups>,
    cached_skin_buffers: Res<CachedSkinBuffers>,
    skin_cache_pipeline_ids: Res<SkinCachePipelineIds>,
    mut render_context: RenderContext,
) {
    let diagnostics = render_context.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(render_context.command_encoder(), "skin caching");

    if !cached_skin_bind_groups.bind_groups.is_empty() {
        let command_encoder = render_context.command_encoder();
        command_encoder.push_debug_group("skin caching");

        // We dispatch one job per bind group (i.e. one job per vertex
        // slab/morph target slab pair).
        for (cached_skin_bind_group_key, cached_skin_bind_group) in
            &cached_skin_bind_groups.bind_groups
        {
            // Fetch the skin task data so that we can compute how many
            // workgroups to dispatch.
            let Some(cached_skin_vertex_buffer_data) = cached_skin_buffers
                .skinned_vertex_buffer_data
                .get(cached_skin_bind_group_key)
            else {
                error!(
                    "We shouldn't have a bind group for the skin cache without the data it came \
                     from"
                );
                continue;
            };

            // Fetch the compute pipeline.
            let Some(pipeline_id) = skin_cache_pipeline_ids
                .vertex_slab_to_pipeline_id
                .get(&cached_skin_bind_group_key.vertex_slab_id)
            else {
                error!("Skin cache pipeline ID should have been created by now");
                continue;
            };
            let Some(pipeline) = pipeline_cache.get_compute_pipeline(*pipeline_id) else {
                // It won't be available while compiling; this is fine.
                continue;
            };

            // Set state.
            let mut compute_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&format!(
                    "skin caching pass, {}",
                    cached_skin_bind_group_key
                )),
                ..default()
            });
            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, Some(&**cached_skin_bind_group), &[]);

            // Dispatch.
            let workgroup_count = cached_skin_vertex_buffer_data
                .current_cached_skinned_vertices_buffer
                .len()
                .div_ceil(SKIN_CACHE_WORKGROUP_SIZE as usize)
                as u32;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        command_encoder.pop_debug_group();
    }

    time_span.end(render_context.command_encoder());
}

/// A resource, part of the render world, that stores all entities with skins
/// and/or morph targets that have [`CacheSkin`] components.
#[derive(Resource, Default)]
pub struct CachedSkinEntities {
    /// All entities that have a skin and a [`CacheSkin`] component.
    pub(crate) skins: MainEntityHashSet,
    /// All entities that have a set of morph targets and a [`CacheSkin`]
    /// component.
    pub(crate) morphs: MainEntityHashSet,
}

/// A resource, part of the render world, that stores the GPU buffers associated
/// with skin caching.
#[derive(Resource)]
pub struct CachedSkinBuffers {
    /// Maps from a bind group key to the task data for that dispatch.
    ///
    /// We have one dispatch per bind group key. The bind group key consists of
    /// the vertex slab ID plus the morph target slab ID if applicable.
    pub skinned_vertex_buffer_data: HashMap<CachedSkinBindGroupKey, SkinTaskSet>,
    /// Maps from the entity containing a mesh instance to its first cached
    /// skinned vertex index in the buffer.
    pub mesh_instance_to_cached_skin_location: MainEntityHashMap<CachedSkinLocation>,
    /// A dummy buffer used when invoking the skin caching shader for meshes
    /// that have no skins (i.e. that only have morph targets).
    pub dummy_skinned_vertex_buffer: Buffer,
    /// A dummy buffer used to substitute for the morph weights buffer in the
    /// skin caching shader for meshes that have no morph targets.
    pub dummy_morph_weight_buffer: Buffer,
    /// A dummy buffer used to substitute for the morph descriptors buffer in
    /// the skin caching shader for meshes that have no morph targets.
    pub dummy_morph_descriptor_buffer: Buffer,
    /// A dummy buffer used to substitute for the morph attributes buffer in the
    /// skin caching shader for meshes that have no morph targets.
    pub dummy_morph_attribute_buffer: Buffer,
}

/// A key that uniquely identifies a single dispatch of the skin caching compute
/// shader.
///
/// We perform one dispatch per (vertex slab, morph target slab) pair with
/// meshes for which one or more instances participate in skin caching.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CachedSkinBindGroupKey {
    /// The vertex slab ID.
    pub vertex_slab_id: MeshSlabId,
    /// The morph target slab ID, if any.
    ///
    /// If this is `None`, then the meshes in question have no morph targets.
    pub morph_target_slab_id: Option<MeshSlabId>,
}

impl CachedSkinBindGroupKey {
    /// Creates a new [`CachedSkinBindGroupKey`] corresponding to the given
    /// vertex slab and morph target slab.
    ///
    /// If `morph_target_slab_id` is `None`, the skin caching shader won't
    /// evaluate morph targets.
    pub fn new(vertex_slab_id: MeshSlabId, morph_target_slab_id: Option<MeshSlabId>) -> Self {
        Self {
            vertex_slab_id,
            morph_target_slab_id,
        }
    }
}

impl Display for CachedSkinBindGroupKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "vertex slab {}", self.vertex_slab_id)?;
        if let Some(morph_target_slab_id) = self.morph_target_slab_id {
            write!(f, ", morph target slab {}", morph_target_slab_id)?;
        }
        Ok(())
    }
}

/// Where the skinned vertex data of a mesh instance that participates in skin
/// caching is located in the cached skinned mesh buffers.
#[derive(Clone, Copy)]
pub struct CachedSkinLocation {
    /// The offset in elements of the skinned vertex data in the skinned mesh
    /// buffer.
    pub current: u32,
    /// The offset in elements of the skinned vertex data in the previous
    /// frame's mesh buffer, if applicable.
    pub prev: Option<NonMaxU32>,
}

impl FromWorld for CachedSkinBuffers {
    fn from_world(world: &mut World) -> CachedSkinBuffers {
        let render_device = world.resource::<RenderDevice>();

        CachedSkinBuffers {
            skinned_vertex_buffer_data: HashMap::default(),
            mesh_instance_to_cached_skin_location: MainEntityHashMap::default(),

            // Initialize dummy buffers.
            dummy_skinned_vertex_buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some("skin caching dummy skinned vertex buffer"),
                size: size_of::<GpuCachedSkinnedVertex>() as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            dummy_morph_weight_buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some("skin caching dummy morph weight buffer"),
                size: size_of::<f32>() as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            dummy_morph_descriptor_buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some("skin caching dummy morph descriptor buffer"),
                size: size_of::<GpuMorphDescriptor>() as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            dummy_morph_attribute_buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some("skin caching dummy morph attribute buffer"),
                size: size_of::<MorphAttributes>() as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        }
    }
}

impl CachedSkinBuffers {
    /// Returns the position of the cached skin within the current and previous
    /// cached skin buffers for the given entity, if applicable.
    pub fn cached_skin_location(&self, main_entity: MainEntity) -> Option<&CachedSkinLocation> {
        self.mesh_instance_to_cached_skin_location.get(&main_entity)
    }

    /// Returns the dummy buffer if there's no buffer for the given vertex slab.
    pub fn buffers_for_key_or_dummy(&'_ self, key: CachedSkinBindGroupKey) -> SkinCacheBuffers<'_> {
        let Some(skinned_vertex_buffer_data) = self.skinned_vertex_buffer_data.get(&key) else {
            return SkinCacheBuffers::new(
                &self.dummy_skinned_vertex_buffer,
                &self.dummy_skinned_vertex_buffer,
            );
        };
        SkinCacheBuffers {
            current: skinned_vertex_buffer_data
                .current_cached_skinned_vertices_buffer
                .buffer()
                .unwrap_or(&self.dummy_skinned_vertex_buffer),
            prev: skinned_vertex_buffer_data
                .prev_cached_skinned_vertices_buffer
                .buffer()
                .unwrap_or(&self.dummy_skinned_vertex_buffer),
        }
    }
}

/// References to the buffers that contain cached skinned vertices for the mesh
/// instances corresponding to a vertex/morph target slab pair.
#[derive(Clone, Copy)]
pub struct SkinCacheBuffers<'a> {
    /// The buffer that stores the current frame's cached skinned vertices.
    pub current: &'a Buffer,
    /// The buffer that stores the previous frame's cached skinned vertices.
    pub prev: &'a Buffer,
}

impl<'a> SkinCacheBuffers<'a> {
    /// Creates a new [`SkinCacheBuffers`] instance referencing the given
    /// buffers.
    fn new(current: &'a Buffer, prev: &'a Buffer) -> SkinCacheBuffers<'a> {
        SkinCacheBuffers { current, prev }
    }
}
/// A resource, part of the render world, that stores bind groups for each key
/// (i.e. each vertex slab/morph target slab pair).
#[derive(Resource, Default)]
pub struct CachedSkinBindGroups {
    /// The bind groups for each key.
    pub bind_groups: HashMap<CachedSkinBindGroupKey, BindGroup>,
}

/// Data specific to one dispatch of the skin caching compute shader.
///
/// There's one such value per (vertex slab ID, morph target slab ID) pair.
pub struct SkinTaskSet {
    /// The GPU buffer that stores the skin tasks.
    ///
    /// Each skin task specifies the mesh instance that is to be skinned and/or
    /// morphed.
    skin_tasks_buffer: RawBufferVec<GpuSkinTask>,

    /// The GPU buffer that stores the skinned/morphed vertices.
    ///
    /// The skin caching compute shader writes into this buffer, and the various
    /// vertex shaders used for rendering read from this buffer.
    current_cached_skinned_vertices_buffer: UninitBufferVec<GpuCachedSkinnedVertex>,

    prev_cached_skinned_vertices_buffer: UninitBufferVec<GpuCachedSkinnedVertex>,
}

impl Default for SkinTaskSet {
    fn default() -> SkinTaskSet {
        SkinTaskSet {
            skin_tasks_buffer: RawBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE),
            current_cached_skinned_vertices_buffer: UninitBufferVec::new(
                BufferUsages::COPY_DST | BufferUsages::STORAGE,
            ),
            prev_cached_skinned_vertices_buffer: UninitBufferVec::new(
                BufferUsages::COPY_DST | BufferUsages::STORAGE,
            ),
        }
    }
}

/// A resource, part of the render world, that stores information needed to
/// construct the compute pipeline for the skin caching shader.
#[derive(Resource)]
pub struct SkinCachePipeline {
    /// The layout of the bind group for that shader.
    pub bind_group_layout: BindGroupLayoutDescriptor,
    /// A handle to the shader itself.
    pub shader: Handle<Shader>,
}

/// A render-world resource that stores the pipeline ID for each vertex slab.
///
/// Note that there only needs to be a separate pipeline per vertex slab, not
/// per morph target slab. That's because all morph displacement buffers have a
/// uniform layout.
#[derive(Resource, Default)]
pub struct SkinCachePipelineIds {
    /// The mapping from the vertex slab ID to the skin caching compute shader
    /// pipeline ID.
    pub vertex_slab_to_pipeline_id: HashMap<MeshSlabId, CachedComputePipelineId>,
}

impl FromWorld for SkinCachePipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let bind_group_layout = BindGroupLayoutDescriptor::new(
            "skin caching bind group layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // @group(0) @binding(0) var<storage> skin_tasks:
                    // array<SkinTask>;
                    storage_buffer_read_only::<GpuSkinTask>(false),
                    // @group(0) @binding(1) var<storage, read_write>
                    // cached_skinned_vertices: array<CachedSkinnedVertex>;
                    storage_buffer::<GpuCachedSkinnedVertex>(false),
                    // @group(0) @binding(2) var<storage> unskinned_vertices:
                    // array<f32>;
                    storage_buffer_read_only::<f32>(false),
                    // @group(0) @binding(3) var<storage> mesh: array<Mesh>;
                    storage_buffer_read_only::<MeshInputUniform>(false),
                    // @group(0) @binding(4) var<storage> joint_matrices:
                    // array<mat4x4<f32>>;
                    storage_buffer_read_only::<Mat4>(false),
                    // @group(0) @binding(5) var<storage> morph_weights:
                    // array<f32>;
                    storage_buffer_read_only::<f32>(false),
                    // @group(0) @binding(6) var<storage> morph_targets:
                    // array<MorphAttributes>;
                    storage_buffer_read_only::<MorphAttributes>(false),
                    // @group(0) @binding(7) var<storage> morph_descriptors:
                    // array<MorphDescriptor>
                    storage_buffer_read_only::<GpuMorphDescriptor>(false),
                ),
            ),
        );

        let shader = load_embedded_asset!(asset_server, "skin_cache.wgsl");

        SkinCachePipeline {
            bind_group_layout,
            shader,
        }
    }
}

/// Uniquely identifies a skin caching compute shader pipeline.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SkinCachePipelineKey {
    /// The vertex buffer layout for the unskinned vertices.
    pub mesh_vertex_buffer_layout: MeshVertexBufferLayoutRef,
}

impl SpecializedComputePipeline for SkinCachePipeline {
    type Key = SkinCachePipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        // `skinning.wgsl` gates some essential definitions behind
        // `SKINNED_OR_MORPHED`, so make sure to define that symbol.
        let mut shader_defs: Vec<ShaderDefVal> =
            vec![ShaderDefVal::Bool("SKINNED_OR_MORPHED".to_owned(), true)];

        // Define symbols corresponding to the location of each attribute.  It's
        // not really feasible in Naga/`naga_oil` at the moment to define a
        // structure that could cover every single possible vertex layout, so we
        // treat the vertex buffers as a bag of words and fetch individual
        // fields dynamically.
        let vertex_buffer_layout = &key.mesh_vertex_buffer_layout.0;
        let attribute_ids = vertex_buffer_layout.attribute_ids();
        let attributes = &vertex_buffer_layout.layout().attributes;
        for i in 0..attribute_ids.len().min(attributes.len()) {
            let (attribute_id, attribute): (&MeshVertexAttributeId, &VertexAttribute) =
                (&attribute_ids[i], &attributes[i]);
            if *attribute_id == Mesh::ATTRIBUTE_POSITION.id {
                debug_assert_eq!(attribute.offset % 4, 0);
                shader_defs.push(ShaderDefVal::UInt(
                    "VERTEX_POSITION_OFFSET".to_owned(),
                    (attribute.offset / 4) as u32,
                ));
            } else if *attribute_id == Mesh::ATTRIBUTE_NORMAL.id {
                debug_assert_eq!(attribute.offset % 4, 0);
                shader_defs.push("VERTEX_NORMALS".into());
                shader_defs.push(ShaderDefVal::UInt(
                    "VERTEX_NORMAL_OFFSET".to_owned(),
                    (attribute.offset / 4) as u32,
                ));
            } else if *attribute_id == Mesh::ATTRIBUTE_TANGENT.id {
                debug_assert_eq!(attribute.offset % 4, 0);
                shader_defs.push("VERTEX_TANGENTS".into());
                shader_defs.push(ShaderDefVal::UInt(
                    "VERTEX_TANGENT_OFFSET".to_owned(),
                    (attribute.offset / 4) as u32,
                ));
            } else if *attribute_id == Mesh::ATTRIBUTE_JOINT_WEIGHT.id {
                debug_assert_eq!(attribute.offset % 4, 0);
                shader_defs.push("VERTEX_JOINT_WEIGHTS".into());
                shader_defs.push(ShaderDefVal::UInt(
                    "VERTEX_JOINT_WEIGHT_OFFSET".to_owned(),
                    (attribute.offset / 4) as u32,
                ));
            } else if *attribute_id == Mesh::ATTRIBUTE_JOINT_INDEX.id {
                debug_assert_eq!(attribute.offset % 4, 0);
                shader_defs.push("VERTEX_JOINT_INDICES".into());
                shader_defs.push(ShaderDefVal::UInt(
                    "VERTEX_JOINT_INDEX_OFFSET".to_owned(),
                    (attribute.offset / 4) as u32,
                ));
            }
        }
        shader_defs.push(ShaderDefVal::UInt(
            "VERTEX_STRIDE".to_owned(),
            (vertex_buffer_layout.layout().array_stride / 4) as u32,
        ));

        ComputePipelineDescriptor {
            label: Some("skin cache pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            immediate_size: 0,
            shader: self.shader.clone(),
            shader_defs,
            entry_point: Some("main".into()),
            zero_initialize_workgroup_memory: true,
        }
    }
}

/// GPU data that describes a single mesh instance.
#[derive(Clone, Copy, ShaderType, Pod, Zeroable)]
#[repr(C)]
struct GpuSkinTask {
    /// The index of the [`MeshInputUniform`] in the buffer for this mesh
    /// instance.
    mesh_input_index: u32,
}

/// The on-GPU layout of a vertex after skinning/morphing.
#[derive(Clone, Copy, Default, ShaderType, Pod, Zeroable)]
#[repr(C)]
pub struct GpuCachedSkinnedVertex {
    /// The post-deformation position of the vertex.
    position: Vec3,
    /// Padding.
    pad_a: u32,
    /// The post-deformation normal of the vertex.
    normal: Vec3,
    /// Padding.
    pad_b: u32,
    /// The post-deformation tangent of the vertex (in mikktspace).
    tangent: Vec4,
}

/// A system that generates skin tasks for each mesh instance that needs to be
/// skinned/morphed via the skin caching shader and otherwise prepares the
/// buffers.
pub fn prepare_skin_cache_buffers(
    cached_skin_buffers: ResMut<CachedSkinBuffers>,
    cached_skin_entities: Res<CachedSkinEntities>,
    mesh_allocator: Res<MeshAllocator>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    mut sorted_cached_skin_entities: Local<Vec<MainEntity>>,
) {
    let cached_skin_buffers = cached_skin_buffers.into_inner();

    let RenderMeshInstances::GpuBuilding(ref render_mesh_instances_gpu) = *render_mesh_instances
    else {
        return;
    };

    // Swap the skinned vertices buffers corresponding to the current and
    // previous frame.
    for skin_task_set in cached_skin_buffers.skinned_vertex_buffer_data.values_mut() {
        mem::swap(
            &mut skin_task_set.current_cached_skinned_vertices_buffer,
            &mut skin_task_set.prev_cached_skinned_vertices_buffer,
        );
        skin_task_set.current_cached_skinned_vertices_buffer.clear();
    }

    // Note all the bind group keys we've seen so that we can expire unused ones
    // at the end of this function.
    let mut all_seen_bind_group_keys: HashSet<_> = HashSet::default();

    // Create a list of all the entities to be skinned/morphed. Entities can be
    // simultaneously skinned *and* morphed, so make sure to dedup so we only
    // process each entity once.
    sorted_cached_skin_entities.clear();
    sorted_cached_skin_entities.extend(
        cached_skin_entities.skins.iter().copied().chain(
            cached_skin_entities
                .morphs
                .iter()
                .filter(|entity| !cached_skin_entities.skins.contains(*entity))
                .copied(),
        ),
    );
    sorted_cached_skin_entities.sort_unstable_by_key(|main_entity| match render_mesh_instances_gpu
        .get(main_entity)
    {
        Some(render_mesh_instance) => render_mesh_instance.gpu_specific.current_uniform_index(),
        None => u32::MAX,
    });

    // Loop over each entity to be skinned/morphed.
    for &main_entity in sorted_cached_skin_entities.iter() {
        let Some(mesh_id) = render_mesh_instances.mesh_asset_id(main_entity) else {
            continue;
        };
        let Some(mesh_slabs) = mesh_allocator.mesh_slabs(&mesh_id) else {
            continue;
        };

        // Get or create a `SkinTaskSet`.
        let bind_group_key = CachedSkinBindGroupKey {
            vertex_slab_id: mesh_slabs.vertex_slab_id,
            morph_target_slab_id: mesh_slabs.morph_target_slab_id,
        };
        all_seen_bind_group_keys.insert(bind_group_key);
        let skin_task_set = cached_skin_buffers
            .skinned_vertex_buffer_data
            .entry(bind_group_key)
            .or_insert_with(default);

        // Fetch the mesh info.
        let (Some(render_mesh_instance), Some(render_mesh)) = (
            render_mesh_instances_gpu.get(&main_entity),
            render_meshes.get(mesh_id),
        ) else {
            continue;
        };

        // Create the skin task.
        skin_task_set.skin_tasks_buffer.push(GpuSkinTask {
            mesh_input_index: render_mesh_instance.gpu_specific.current_uniform_index(),
        });
        let current_first_vertex_index =
            skin_task_set.current_cached_skinned_vertices_buffer.len() as u32;

        match cached_skin_buffers
            .mesh_instance_to_cached_skin_location
            .entry(main_entity)
        {
            Entry::Occupied(mut occupied_entry) => {
                occupied_entry.get_mut().prev = NonMaxU32::new(occupied_entry.get().current);
                occupied_entry.get_mut().current = current_first_vertex_index;
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(CachedSkinLocation {
                    current: current_first_vertex_index,
                    prev: None,
                });
            }
        }

        // Reserve space for the skinned output vertices.
        skin_task_set
            .current_cached_skinned_vertices_buffer
            .add_multiple(render_mesh.vertex_count as usize);
    }

    // Expire buffers and records corresponding to mesh instances that have been
    // removed.
    cached_skin_buffers
        .skinned_vertex_buffer_data
        .retain(|bind_group_key, _| all_seen_bind_group_keys.contains(bind_group_key));
    cached_skin_buffers
        .mesh_instance_to_cached_skin_location
        .retain(|mesh_instance, _| {
            cached_skin_entities.skins.contains(mesh_instance)
                || cached_skin_entities.morphs.contains(mesh_instance)
        });
}

pub fn write_skin_cache_buffers(
    mut cached_skin_buffers: ResMut<CachedSkinBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for skin_task_set in cached_skin_buffers.skinned_vertex_buffer_data.values_mut() {
        debug_assert!(!skin_task_set.skin_tasks_buffer.is_empty());
        skin_task_set
            .skin_tasks_buffer
            .write_buffer(&render_device, &render_queue);

        // Should never happen, but just in case…
        if skin_task_set
            .current_cached_skinned_vertices_buffer
            .is_empty()
        {
            skin_task_set.current_cached_skinned_vertices_buffer.add();
        }
        skin_task_set
            .current_cached_skinned_vertices_buffer
            .write_buffer(&render_device);
    }
}

/// A system that creates all the bind groups necessary for each skin caching
/// job.
pub fn prepare_skin_cache_bind_groups(
    mut cached_skin_bind_groups: ResMut<CachedSkinBindGroups>,
    batched_instance_buffers: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    cached_skin_buffers: ResMut<CachedSkinBuffers>,
    pipeline_cache: Res<PipelineCache>,
    skin_cache_pipeline: Res<SkinCachePipeline>,
    mesh_allocator: Res<MeshAllocator>,
    skin_uniforms: Res<SkinUniforms>,
    morph_uniforms: Res<MorphUniforms>,
    render_device: Res<RenderDevice>,
) {
    // If we don't have any meshes, bail.
    let Some(mesh_input_uniform_buffer) = batched_instance_buffers
        .current_input_buffer
        .buffer()
        .buffer()
    else {
        return;
    };

    let cached_skin_buffers = cached_skin_buffers.into_inner();

    cached_skin_bind_groups.bind_groups.clear();

    let bind_group_layout =
        pipeline_cache.get_bind_group_layout(&skin_cache_pipeline.bind_group_layout);

    // Loop over each skin task set: i.e. each (vertex slab ID, morph target
    // slab ID) pair.
    for (cached_skin_bind_group_key, skin_task_set) in
        &mut cached_skin_buffers.skinned_vertex_buffer_data
    {
        debug_assert!(!skin_task_set.skin_tasks_buffer.is_empty());

        // Unpack the vertex slab, the skin tasks buffer, and the
        // destination cached skinned vertices.
        let (
            Some(vertex_slab_buffer),
            Some(skin_tasks_buffer),
            Some(cached_skinned_vertices_buffer),
        ) = (
            mesh_allocator.buffer_for_slab(cached_skin_bind_group_key.vertex_slab_id),
            skin_task_set.skin_tasks_buffer.buffer(),
            skin_task_set
                .current_cached_skinned_vertices_buffer
                .buffer(),
        )
        else {
            error!("The skin caching buffers should have been uploaded by now");
            continue;
        };

        // Get the morph-target-related buffers, or dummy buffers if not
        // applicable.
        let (morph_weights_buffer, morph_descriptors_buffer, morph_attributes_buffer) = (
            morph_uniforms
                .current_buffer
                .buffer()
                .unwrap_or(&cached_skin_buffers.dummy_morph_weight_buffer),
            morph_uniforms
                .as_ref()
                .descriptors_buffer
                .as_ref()
                .and_then(|descriptors_buffer| descriptors_buffer.buffer())
                .unwrap_or(&cached_skin_buffers.dummy_morph_descriptor_buffer),
            cached_skin_bind_group_key
                .morph_target_slab_id
                .and_then(|morph_target_slab_id| {
                    mesh_allocator.buffer_for_slab(morph_target_slab_id)
                })
                .unwrap_or(&cached_skin_buffers.dummy_morph_attribute_buffer),
        );

        // Create the bind group.
        let bind_group = SkinCachingBindGroupInfo {
            render_device: &render_device,
            bind_group_key: *cached_skin_bind_group_key,
            bind_group_layout: &bind_group_layout,
            skin_tasks_buffer,
            skin_tasks_buffer_len: skin_task_set.skin_tasks_buffer.len(),
            cached_skinned_vertices_buffer,
            cached_skinned_vertices_buffer_len: skin_task_set
                .current_cached_skinned_vertices_buffer
                .len(),
            vertex_slab_buffer,
            mesh_input_uniform_buffer,
            skin_uniforms_buffer: &skin_uniforms.current_buffer,
            morph_weights_buffer,
            morph_attributes_buffer,
            morph_descriptors_buffer,
        }
        .create_bind_group();

        // Record the bind group.
        cached_skin_bind_groups
            .bind_groups
            .insert(*cached_skin_bind_group_key, bind_group);
    }
}

/// Data needed to construct a bind group for the skin caching shader.
struct SkinCachingBindGroupInfo<'a> {
    /// The render device.
    render_device: &'a RenderDevice,
    /// The IDs of the vertex buffer slab and morph target slab (if applicable).
    bind_group_key: CachedSkinBindGroupKey,
    /// The layout for the bind group.
    bind_group_layout: &'a BindGroupLayout,
    /// The buffer containing the skin tasks.
    skin_tasks_buffer: &'a Buffer,
    /// The length of the buffer containing the skin tasks.
    ///
    /// This must be the actual length, not the length of the allocation.
    skin_tasks_buffer_len: usize,
    /// The output buffer where the skinned vertices are to be stored.
    cached_skinned_vertices_buffer: &'a Buffer,
    /// The length of the output buffer where the skinned vertices are to be
    /// stored.
    ///
    /// This must be the actual length, not the length of the allocation.
    cached_skinned_vertices_buffer_len: usize,
    /// The slab containing the vertices.
    vertex_slab_buffer: &'a Buffer,
    /// The buffer containing the [`MeshInputUniform`]s.
    mesh_input_uniform_buffer: &'a Buffer,
    /// The buffer containing the skin uniforms.
    skin_uniforms_buffer: &'a Buffer,
    /// The buffer containing the morph weights, or a dummy buffer if there are
    /// no morph targets to be processed.
    morph_weights_buffer: &'a Buffer,
    /// The buffer containing the morph attributes, or a dummy buffer if there
    /// are no morph targets to be processed.
    morph_attributes_buffer: &'a Buffer,
    /// The buffer containing the morph descriptors, or a dummy buffer if there
    /// are no morph targets to be processed.
    morph_descriptors_buffer: &'a Buffer,
}

impl<'a> SkinCachingBindGroupInfo<'a> {
    fn create_bind_group(self) -> BindGroup {
        self.render_device.create_bind_group(
            Some(&*format!(
                "skin caching bind group ({})",
                self.bind_group_key
            )),
            self.bind_group_layout,
            &BindGroupEntries::sequential((
                // NB: Make sure these are tightly bound!
                // @group(0) @binding(0) var<storage> skin_tasks:
                // array<SkinTask>;
                BufferBinding {
                    buffer: self.skin_tasks_buffer,
                    offset: 0,
                    size: Some(
                        NonZero::try_from(
                            self.skin_tasks_buffer_len as u64 * size_of::<GpuSkinTask>() as u64,
                        )
                        .unwrap(),
                    ),
                },
                // @group(0) @binding(1) var<storage, read_write>
                // cached_skinned_vertices: array<CachedSkinnedVertex>;
                BufferBinding {
                    buffer: self.cached_skinned_vertices_buffer,
                    offset: 0,
                    size: Some(
                        NonZero::try_from(
                            self.cached_skinned_vertices_buffer_len as u64
                                * size_of::<GpuCachedSkinnedVertex>() as u64,
                        )
                        .unwrap(),
                    ),
                },
                // @group(0) @binding(2) var<storage> unskinned_vertices:
                // array<f32>;
                self.vertex_slab_buffer.as_entire_binding(),
                // @group(0) @binding(3) var<storage> meshes: array<MeshInput>;
                self.mesh_input_uniform_buffer.as_entire_binding(),
                // @group(0) @binding(4) var<storage> joint_matrices: array<mat4x4<f32>>;
                self.skin_uniforms_buffer.as_entire_binding(),
                // @group(0) @binding(5) var<storage> morph_weights: array<f32>;
                self.morph_weights_buffer.as_entire_binding(),
                // @group(0) @binding(6) var<storage> morph_targets:
                // array<MorphAttributes>;
                self.morph_attributes_buffer.as_entire_binding(),
                // @group(0) @binding(7) var<storage> morph_descriptors:
                // array<MorphDescriptor>;
                self.morph_descriptors_buffer.as_entire_binding(),
            )),
        )
    }
}

/// A system that creates needed compute pipelines for all skin caching jobs
/// taking place this frame.
pub fn prepare_skin_cache_pipelines(
    mut skin_cache_pipeline_ids: ResMut<SkinCachePipelineIds>,
    mut skin_cache_pipelines: ResMut<SpecializedComputePipelines<SkinCachePipeline>>,
    skin_cache_pipeline: Res<SkinCachePipeline>,
    pipeline_cache: Res<PipelineCache>,
    skin_uniforms: Res<SkinUniforms>,
    morph_indices: Res<MorphIndices>,
    mesh_allocator: Res<MeshAllocator>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
) {
    let MorphIndices::Storage {
        ref morph_weights_info,
        ..
    } = *morph_indices
    else {
        return;
    };

    // Build up a mapping from each vertex slab ID to its associated layout.
    let mut vertex_slab_id_to_vertex_layout = HashMap::new();
    for main_entity in skin_uniforms
        .skin_uniform_info
        .keys()
        .chain(morph_weights_info.keys())
    {
        let Some(mesh_id) = render_mesh_instances.mesh_asset_id(*main_entity) else {
            continue;
        };
        let Some(mesh_slabs) = mesh_allocator.mesh_slabs(&mesh_id) else {
            continue;
        };
        if let Entry::Vacant(vacant_entry) =
            vertex_slab_id_to_vertex_layout.entry(mesh_slabs.vertex_slab_id)
        {
            let Some(render_mesh) = render_meshes.get(mesh_id) else {
                error!("Mesh instance wasn't found in `RenderMesh`");
                continue;
            };
            vacant_entry.insert(render_mesh.layout.clone());
        }
    }

    skin_cache_pipeline_ids.vertex_slab_to_pipeline_id.clear();

    // Now that we know all the layouts, specialize the pipelines.
    for (slab_id, mesh_vertex_buffer_layout) in vertex_slab_id_to_vertex_layout {
        let pipeline_id = skin_cache_pipelines.specialize(
            &pipeline_cache,
            &skin_cache_pipeline,
            SkinCachePipelineKey {
                mesh_vertex_buffer_layout,
            },
        );
        skin_cache_pipeline_ids
            .vertex_slab_to_pipeline_id
            .insert(slab_id, pipeline_id);
    }
}
