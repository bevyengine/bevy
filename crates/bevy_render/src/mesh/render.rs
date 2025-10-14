use bevy_asset::AssetId;
use bevy_camera::visibility::RenderLayers;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_encase_derive::ShaderType;
use bevy_math::{Affine3, Rect, UVec2, Vec3, Vec4};
use bevy_mesh::{Mesh, Mesh3d, MeshTag};
use bevy_render::batching::gpu_preprocessing::InstanceInputUniformBuffer;
use bevy_transform::components::GlobalTransform;
use bevy_utils::default;
use glam::Affine3A;

use bevy_render::sync_world::{MainEntity, MainEntityHashMap};

use bytemuck::{Pod, Zeroable};
use nonmax::{NonMaxU16, NonMaxU32};

use crate::{
    lightmap::{pack_lightmap_uv_rect, LightmapSlabIndex, LightmapSlotIndex},
    mesh::material_bind_group::{MaterialBindGroupSlot, MaterialBindingId},
    render_phase::InputUniformIndex,
};

#[derive(Component)]
pub struct MeshTransforms {
    pub world_from_local: Affine3,
    pub previous_world_from_local: Affine3,
    pub flags: u32,
}

#[derive(ShaderType, Clone)]
pub struct MeshUniform {
    // Affine 4x3 matrices transposed to 3x4
    pub world_from_local: [Vec4; 3],
    pub previous_world_from_local: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub local_from_world_transpose_a: [Vec4; 2],
    pub local_from_world_transpose_b: f32,
    pub flags: u32,
    // Four 16-bit unsigned normalized UV values packed into a `UVec2`:
    //
    //                         <--- MSB                   LSB --->
    //                         +---- min v ----+ +---- min u ----+
    //     lightmap_uv_rect.x: vvvvvvvv vvvvvvvv uuuuuuuu uuuuuuuu,
    //                         +---- max v ----+ +---- max u ----+
    //     lightmap_uv_rect.y: VVVVVVVV VVVVVVVV UUUUUUUU UUUUUUUU,
    //
    // (MSB: most significant bit; LSB: least significant bit.)
    pub lightmap_uv_rect: UVec2,
    /// The index of this mesh's first vertex in the vertex buffer.
    ///
    /// Multiple meshes can be packed into a single vertex buffer (see
    /// [`MeshAllocator`](`crate::mesh::allocator::MeshAllocator`) ). This value stores the offset of the first vertex in
    /// this mesh in that buffer.
    pub first_vertex_index: u32,
    /// The current skin index, or `u32::MAX` if there's no skin.
    pub current_skin_index: u32,
    /// The material and lightmap indices, packed into 32 bits.
    ///
    /// Low 16 bits: index of the material inside the bind group data.
    /// High 16 bits: index of the lightmap in the binding array.
    pub material_and_lightmap_bind_group_slot: u32,
    /// User supplied tag to identify this mesh instance.
    pub tag: u32,
    /// Padding.
    pub pad: u32,
}

/// Information that has to be transferred from CPU to GPU in order to produce
/// the full [`MeshUniform`].
///
/// This is essentially a subset of the fields in [`MeshUniform`] above.
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Default, Debug)]
#[repr(C)]
pub struct MeshInputUniform {
    /// Affine 4x3 matrix transposed to 3x4.
    pub world_from_local: [Vec4; 3],
    /// Four 16-bit unsigned normalized UV values packed into a `UVec2`:
    ///
    /// ```text
    ///                         <--- MSB                   LSB --->
    ///                         +---- min v ----+ +---- min u ----+
    ///     lightmap_uv_rect.x: vvvvvvvv vvvvvvvv uuuuuuuu uuuuuuuu,
    ///                         +---- max v ----+ +---- max u ----+
    ///     lightmap_uv_rect.y: VVVVVVVV VVVVVVVV UUUUUUUU UUUUUUUU,
    ///
    /// (MSB: most significant bit; LSB: least significant bit.)
    /// ```
    pub lightmap_uv_rect: UVec2,
    /// Various [`MeshFlags`].
    pub flags: u32,
    /// The index of this mesh's [`MeshInputUniform`] in the previous frame's
    /// buffer, if applicable.
    ///
    /// This is used for TAA. If not present, this will be `u32::MAX`.
    pub previous_input_index: u32,
    /// The index of this mesh's first vertex in the vertex buffer.
    ///
    /// Multiple meshes can be packed into a single vertex buffer (see
    /// [`MeshAllocator`](`crate::mesh::allocator::MeshAllocator`) ). This value stores the offset of the first vertex in
    /// this mesh in that buffer.
    pub first_vertex_index: u32,
    /// The index of this mesh's first index in the index buffer, if any.
    ///
    /// Multiple meshes can be packed into a single index buffer (see
    /// [`MeshAllocator`](`crate::mesh::allocator::MeshAllocator`) ). This value stores the offset of the first index in
    /// this mesh in that buffer.
    ///
    /// If this mesh isn't indexed, this value is ignored.
    pub first_index_index: u32,
    /// For an indexed mesh, the number of indices that make it up; for a
    /// non-indexed mesh, the number of vertices in it.
    pub index_count: u32,
    /// The current skin index, or `u32::MAX` if there's no skin.
    pub current_skin_index: u32,
    /// The material and lightmap indices, packed into 32 bits.
    ///
    /// Low 16 bits: index of the material inside the bind group data.
    /// High 16 bits: index of the lightmap in the binding array.
    pub material_and_lightmap_bind_group_slot: u32,
    /// The number of the frame on which this [`MeshInputUniform`] was built.
    ///
    /// This is used to validate the previous transform and skin. If this
    /// [`MeshInputUniform`] wasn't updated on this frame, then we know that
    /// neither this mesh's transform nor that of its joints have been updated
    /// on this frame, and therefore the transforms of both this mesh and its
    /// joints must be identical to those for the previous frame.
    pub timestamp: u32,
    /// User supplied tag to identify this mesh instance.
    pub tag: u32,
    /// Padding.
    pub pad: u32,
}

impl MeshUniform {
    pub fn new(
        mesh_transforms: &MeshTransforms,
        first_vertex_index: u32,
        material_bind_group_slot: MaterialBindGroupSlot,
        maybe_lightmap: Option<(LightmapSlotIndex, Rect)>,
        current_skin_index: Option<u32>,
        tag: Option<u32>,
    ) -> Self {
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            mesh_transforms.world_from_local.inverse_transpose_3x3();
        let lightmap_bind_group_slot = match maybe_lightmap {
            None => u16::MAX,
            Some((slot_index, _)) => slot_index.into(),
        };

        Self {
            world_from_local: mesh_transforms.world_from_local.to_transpose(),
            previous_world_from_local: mesh_transforms.previous_world_from_local.to_transpose(),
            lightmap_uv_rect: pack_lightmap_uv_rect(maybe_lightmap.map(|(_, uv_rect)| uv_rect)),
            local_from_world_transpose_a,
            local_from_world_transpose_b,
            flags: mesh_transforms.flags,
            first_vertex_index,
            current_skin_index: current_skin_index.unwrap_or(u32::MAX),
            material_and_lightmap_bind_group_slot: u32::from(material_bind_group_slot)
                | ((lightmap_bind_group_slot as u32) << 16),
            tag: tag.unwrap_or(0),
            pad: 0,
        }
    }
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_types.wgsl!
bitflags::bitflags! {
    /// Various flags and tightly-packed values on a mesh.
    ///
    /// Flags grow from the top bit down; other values grow from the bottom bit
    /// up.
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        /// Bitmask for the 16-bit index into the LOD array.
        ///
        /// This will be `u16::MAX` if this mesh has no LOD.
        const LOD_INDEX_MASK              = (1 << 16) - 1;
        /// Disables frustum culling for this mesh.
        ///
        /// This corresponds to the
        /// [`bevy_render::view::visibility::NoFrustumCulling`] component.
        const NO_FRUSTUM_CULLING          = 1 << 28;
        const SHADOW_RECEIVER             = 1 << 29;
        const TRANSMITTED_SHADOW_RECEIVER = 1 << 30;
        // Indicates the sign of the determinant of the 3x3 model matrix. If the sign is positive,
        // then the flag should be set, else it should not be set.
        const SIGN_DETERMINANT_MODEL_3X3  = 1 << 31;
        const NONE                        = 0;
        const UNINITIALIZED               = 0xFFFFFFFF;
    }
}

impl MeshFlags {
    pub fn from_components(
        transform: &GlobalTransform,
        lod_index: Option<NonMaxU16>,
        no_frustum_culling: bool,
        not_shadow_receiver: bool,
        transmitted_receiver: bool,
    ) -> MeshFlags {
        let mut mesh_flags = if not_shadow_receiver {
            MeshFlags::empty()
        } else {
            MeshFlags::SHADOW_RECEIVER
        };
        if no_frustum_culling {
            mesh_flags |= MeshFlags::NO_FRUSTUM_CULLING;
        }
        if transmitted_receiver {
            mesh_flags |= MeshFlags::TRANSMITTED_SHADOW_RECEIVER;
        }
        if transform.affine().matrix3.determinant().is_sign_positive() {
            mesh_flags |= MeshFlags::SIGN_DETERMINANT_MODEL_3X3;
        }

        let lod_index_bits = match lod_index {
            None => u16::MAX,
            Some(lod_index) => u16::from(lod_index),
        };
        mesh_flags |=
            MeshFlags::from_bits_retain((lod_index_bits as u32) << MeshFlags::LOD_INDEX_SHIFT);

        mesh_flags
    }

    /// The first bit of the LOD index.
    pub const LOD_INDEX_SHIFT: u32 = 0;
}

bitflags::bitflags! {
    /// Various useful flags for [`RenderMeshInstance`]s.
    #[derive(Clone, Copy)]
    pub struct RenderMeshInstanceFlags: u8 {
        /// The mesh casts shadows.
        const SHADOW_CASTER           = 1 << 0;
        /// The mesh can participate in automatic batching.
        const AUTOMATIC_BATCHING      = 1 << 1;
        /// The mesh had a transform last frame and so is eligible for motion
        /// vector computation.
        const HAS_PREVIOUS_TRANSFORM  = 1 << 2;
        /// The mesh had a skin last frame and so that skin should be taken into
        /// account for motion vector computation.
        const HAS_PREVIOUS_SKIN       = 1 << 3;
        /// The mesh had morph targets last frame and so they should be taken
        /// into account for motion vector computation.
        const HAS_PREVIOUS_MORPH      = 1 << 4;
    }
}

/// CPU data that the render world keeps for each entity, when *not* using GPU
/// mesh uniform building.
#[derive(Deref, DerefMut)]
pub struct RenderMeshInstanceCpu {
    /// Data shared between both the CPU mesh uniform building and the GPU mesh
    /// uniform building paths.
    #[deref]
    pub shared: RenderMeshInstanceShared,
    /// The transform of the mesh.
    ///
    /// This will be written into the [`MeshUniform`] at the appropriate time.
    pub transforms: MeshTransforms,
}

/// CPU data that the render world needs to keep for each entity that contains a
/// mesh when using GPU mesh uniform building.
#[derive(Deref, DerefMut)]
pub struct RenderMeshInstanceGpu {
    /// Data shared between both the CPU mesh uniform building and the GPU mesh
    /// uniform building paths.
    #[deref]
    pub shared: RenderMeshInstanceShared,
    /// The translation of the mesh.
    ///
    /// This is the only part of the transform that we have to keep on CPU (for
    /// distance sorting).
    pub translation: Vec3,
    /// The index of the [`MeshInputUniform`] in the buffer.
    pub current_uniform_index: NonMaxU32,
}

#[derive(Component, PartialEq, Default)]
pub struct PreviousGlobalTransform(pub Affine3A);

/// CPU data that the render world needs to keep about each entity that contains
/// a mesh.
pub struct RenderMeshInstanceShared {
    /// The [`AssetId`] of the mesh.
    pub mesh_asset_id: AssetId<Mesh>,
    /// A slot for the material bind group index.
    pub material_bindings_index: MaterialBindingId,
    /// Various flags.
    pub flags: RenderMeshInstanceFlags,
    /// Index of the slab that the lightmap resides in, if a lightmap is
    /// present.
    pub lightmap_slab_index: Option<LightmapSlabIndex>,
    /// User supplied tag to identify this mesh instance.
    pub tag: u32,
    /// Render layers that this mesh instance belongs to.
    pub render_layers: Option<RenderLayers>,
}

impl RenderMeshInstanceShared {
    /// A gpu builder will provide the mesh instance id
    pub fn for_gpu_building(
        previous_transform: Option<&PreviousGlobalTransform>,
        mesh: &Mesh3d,
        tag: Option<&MeshTag>,
        not_shadow_caster: bool,
        no_automatic_batching: bool,
        render_layers: Option<&RenderLayers>,
    ) -> Self {
        Self::for_cpu_building(
            previous_transform,
            mesh,
            tag,
            default(),
            not_shadow_caster,
            no_automatic_batching,
            render_layers,
        )
    }

    /// The cpu builder does not have an equivalent ?
    pub fn for_cpu_building(
        previous_transform: Option<&PreviousGlobalTransform>,
        mesh: &Mesh3d,
        tag: Option<&MeshTag>,
        material_bindings_index: MaterialBindingId,
        not_shadow_caster: bool,
        no_automatic_batching: bool,
        render_layers: Option<&RenderLayers>,
    ) -> Self {
        let mut mesh_instance_flags = RenderMeshInstanceFlags::empty();
        mesh_instance_flags.set(RenderMeshInstanceFlags::SHADOW_CASTER, !not_shadow_caster);
        mesh_instance_flags.set(
            RenderMeshInstanceFlags::AUTOMATIC_BATCHING,
            !no_automatic_batching,
        );
        mesh_instance_flags.set(
            RenderMeshInstanceFlags::HAS_PREVIOUS_TRANSFORM,
            previous_transform.is_some(),
        );

        RenderMeshInstanceShared {
            mesh_asset_id: mesh.id(),
            flags: mesh_instance_flags,
            material_bindings_index,
            lightmap_slab_index: None,
            tag: tag.map_or(0, |i| **i),
            render_layers: render_layers.cloned(),
        }
    }

    /// Returns true if this entity is eligible to participate in automatic
    /// batching.
    #[inline]
    pub fn should_batch(&self) -> bool {
        self.flags
            .contains(RenderMeshInstanceFlags::AUTOMATIC_BATCHING)
    }
}

/// Information that the render world keeps about each entity that contains a
/// mesh.
///
/// The set of information needed is different depending on whether CPU or GPU
/// [`MeshUniform`] building is in use.
#[derive(Resource)]
pub enum RenderMeshInstances {
    /// Information needed when using CPU mesh instance data building.
    CpuBuilding(RenderMeshInstancesCpu),
    /// Information needed when using GPU mesh instance data building.
    GpuBuilding(RenderMeshInstancesGpu),
}

/// Information that the render world keeps about each entity that contains a
/// mesh, when using CPU mesh instance data building.
#[derive(Default, Deref, DerefMut)]
pub struct RenderMeshInstancesCpu(MainEntityHashMap<RenderMeshInstanceCpu>);

/// Information that the render world keeps about each entity that contains a
/// mesh, when using GPU mesh instance data building.
#[derive(Default, Deref, DerefMut)]
pub struct RenderMeshInstancesGpu(MainEntityHashMap<RenderMeshInstanceGpu>);

impl RenderMeshInstances {
    /// Creates a new [`RenderMeshInstances`] instance.
    pub fn new(use_gpu_instance_buffer_builder: bool) -> RenderMeshInstances {
        if use_gpu_instance_buffer_builder {
            RenderMeshInstances::GpuBuilding(RenderMeshInstancesGpu::default())
        } else {
            RenderMeshInstances::CpuBuilding(RenderMeshInstancesCpu::default())
        }
    }

    /// Returns the ID of the mesh asset attached to the given entity, if any.
    pub fn mesh_asset_id(&self, entity: MainEntity) -> Option<AssetId<Mesh>> {
        match *self {
            RenderMeshInstances::CpuBuilding(ref instances) => instances.mesh_asset_id(entity),
            RenderMeshInstances::GpuBuilding(ref instances) => instances.mesh_asset_id(entity),
        }
    }

    /// Constructs [`RenderMeshQueueData`] for the given entity, if it has a
    /// mesh attached.
    pub fn render_mesh_queue_data(&self, entity: MainEntity) -> Option<RenderMeshQueueData<'_>> {
        match *self {
            RenderMeshInstances::CpuBuilding(ref instances) => {
                instances.render_mesh_queue_data(entity)
            }
            RenderMeshInstances::GpuBuilding(ref instances) => {
                instances.render_mesh_queue_data(entity)
            }
        }
    }

    /// Inserts the given flags into the CPU or GPU render mesh instance data
    /// for the given mesh as appropriate.
    pub fn insert_mesh_instance_flags(
        &mut self,
        entity: MainEntity,
        flags: RenderMeshInstanceFlags,
    ) {
        match *self {
            RenderMeshInstances::CpuBuilding(ref mut instances) => {
                instances.insert_mesh_instance_flags(entity, flags);
            }
            RenderMeshInstances::GpuBuilding(ref mut instances) => {
                instances.insert_mesh_instance_flags(entity, flags);
            }
        }
    }
}

impl RenderMeshInstancesCpu {
    fn mesh_asset_id(&self, entity: MainEntity) -> Option<AssetId<Mesh>> {
        self.get(&entity)
            .map(|render_mesh_instance| render_mesh_instance.mesh_asset_id)
    }

    pub fn render_mesh_queue_data(&self, entity: MainEntity) -> Option<RenderMeshQueueData<'_>> {
        self.get(&entity)
            .map(|render_mesh_instance| RenderMeshQueueData {
                shared: &render_mesh_instance.shared,
                translation: render_mesh_instance.transforms.world_from_local.translation,
                current_uniform_index: InputUniformIndex::default(),
            })
    }

    /// Inserts the given flags into the render mesh instance data for the given
    /// mesh.
    fn insert_mesh_instance_flags(&mut self, entity: MainEntity, flags: RenderMeshInstanceFlags) {
        if let Some(instance) = self.get_mut(&entity) {
            instance.flags.insert(flags);
        }
    }
}

impl RenderMeshInstancesGpu {
    fn mesh_asset_id(&self, entity: MainEntity) -> Option<AssetId<Mesh>> {
        self.get(&entity)
            .map(|render_mesh_instance| render_mesh_instance.mesh_asset_id)
    }

    fn render_mesh_queue_data(&self, entity: MainEntity) -> Option<RenderMeshQueueData<'_>> {
        self.get(&entity)
            .map(|render_mesh_instance| RenderMeshQueueData {
                shared: &render_mesh_instance.shared,
                translation: render_mesh_instance.translation,
                current_uniform_index: InputUniformIndex(
                    render_mesh_instance.current_uniform_index.into(),
                ),
            })
    }

    /// Inserts the given flags into the render mesh instance data for the given
    /// mesh.
    fn insert_mesh_instance_flags(&mut self, entity: MainEntity, flags: RenderMeshInstanceFlags) {
        if let Some(instance) = self.get_mut(&entity) {
            instance.flags.insert(flags);
        }
    }
}

/// Data that systems need in order to place entities that contain meshes in the right batch.
#[derive(Deref)]
pub struct RenderMeshQueueData<'a> {
    /// General information about the mesh instance.
    #[deref]
    pub shared: &'a RenderMeshInstanceShared,
    /// The translation of the mesh instance.
    pub translation: Vec3,
    /// The index of the [`MeshInputUniform`] in the GPU buffer for this mesh
    /// instance.
    pub current_uniform_index: InputUniformIndex,
}

/// Removes a [`MeshInputUniform`] corresponding to an entity that became
/// invisible from the buffer.
pub fn remove_mesh_input_uniform(
    entity: MainEntity,
    render_mesh_instances: &mut MainEntityHashMap<RenderMeshInstanceGpu>,
    current_input_buffer: &mut InstanceInputUniformBuffer<MeshInputUniform>,
) -> Option<u32> {
    // Remove the uniform data.
    let removed_render_mesh_instance = render_mesh_instances.remove(&entity)?;

    let removed_uniform_index = removed_render_mesh_instance.current_uniform_index.get();
    current_input_buffer.remove(removed_uniform_index);
    Some(removed_uniform_index)
}
