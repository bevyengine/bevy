use core::mem::size_of;

use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_render::render_resource::Buffer;
use bevy_render::sync_world::{MainEntity, MainEntityHashMap};
use offset_allocator::{Allocation, Allocator};
use smallvec::SmallVec;

/// Maximum number of joints supported for skinned meshes.
///
/// It is used to allocate buffers.
/// The correctness of the value depends on the GPU/platform.
/// The current value is chosen because it is guaranteed to work everywhere.
/// To allow for bigger values, a check must be made for the limits
/// of the GPU at runtime, which would mean not using consts anymore.
pub const MAX_JOINTS: usize = 256;

/// The total number of joints we support.
///
/// This is 256 GiB worth of joint matrices, which we will never hit under any
/// reasonable circumstances.
pub const MAX_TOTAL_JOINTS: u32 = 1024 * 1024 * 1024;

/// The number of joints that we allocate at a time.
///
/// Some hardware requires that uniforms be allocated on 256-byte boundaries, so
/// we need to allocate 4 64-byte matrices at a time to satisfy alignment
/// requirements.
pub const JOINTS_PER_ALLOCATION_UNIT: u32 = (256 / size_of::<Mat4>()) as u32;

/// The maximum ratio of the number of entities whose transforms changed to the
/// total number of joints before we re-extract all joints.
///
/// We use this as a heuristic to decide whether it's worth switching over to
/// fine-grained detection to determine which skins need extraction. If the
/// number of changed entities is over this threshold, we skip change detection
/// and simply re-extract the transforms of all joints.
pub const JOINT_EXTRACTION_THRESHOLD_FACTOR: f64 = 0.25;

/// The location of the first joint matrix in the skin uniform buffer.
#[derive(Clone, Copy)]
pub struct SkinByteOffset {
    /// The byte offset of the first joint matrix.
    pub byte_offset: u32,
}

impl SkinByteOffset {
    /// Index to be in address space based on the size of a skin uniform.
    const fn from_index(index: usize) -> Self {
        SkinByteOffset {
            byte_offset: (index * size_of::<Mat4>()) as u32,
        }
    }

    /// Returns this skin index in elements (not bytes).
    ///
    /// Each element is a 4x4 matrix.
    pub fn index(&self) -> u32 {
        self.byte_offset / size_of::<Mat4>() as u32
    }
}

/// The GPU buffers containing joint matrices for all skinned meshes.
///
/// This is double-buffered: we store the joint matrices of each mesh for the
/// previous frame in addition to those of each mesh for the current frame. This
/// is for motion vector calculation. Every frame, we swap buffers and overwrite
/// the joint matrix buffer from two frames ago with the data for the current
/// frame.
///
/// Notes on implementation: see comment on top of the `extract_skins` system.
#[derive(Resource)]
pub struct SkinUniforms {
    /// The CPU-side buffer that stores the joint matrices for skinned meshes in
    /// the current frame.
    pub current_staging_buffer: Vec<Mat4>,
    /// The GPU-side buffer that stores the joint matrices for skinned meshes in
    /// the current frame.
    pub current_buffer: Buffer,
    /// The GPU-side buffer that stores the joint matrices for skinned meshes in
    /// the previous frame.
    pub prev_buffer: Buffer,
    /// The offset allocator that manages the placement of the joints within the
    /// [`Self::current_buffer`].
    pub allocator: Allocator,
    /// Allocation information that we keep about each skin.
    pub skin_uniform_info: MainEntityHashMap<SkinUniformInfo>,
    /// Maps each joint entity to the skins it's associated with.
    ///
    /// We use this in conjunction with change detection to only update the
    /// skins that need updating each frame.
    ///
    /// Note that conceptually this is a hash map of sets, but we use a
    /// [`SmallVec`] to avoid allocations for the vast majority of the cases in
    /// which each bone belongs to exactly one skin.
    pub joint_to_skins: MainEntityHashMap<SmallVec<[MainEntity; 1]>>,
    /// The total number of joints in the scene.
    ///
    /// We use this as part of our heuristic to decide whether to use
    /// fine-grained change detection.
    pub total_joints: usize,
}

impl SkinUniforms {
    /// Returns the current offset in joints of the skin in the buffer.
    pub fn skin_index(&self, skin: MainEntity) -> Option<u32> {
        self.skin_uniform_info
            .get(&skin)
            .map(SkinUniformInfo::offset)
    }

    /// Returns the current offset in bytes of the skin in the buffer.
    pub fn skin_byte_offset(&self, skin: MainEntity) -> Option<SkinByteOffset> {
        self.skin_uniform_info.get(&skin).map(|skin_uniform_info| {
            SkinByteOffset::from_index(skin_uniform_info.offset() as usize)
        })
    }

    /// Returns an iterator over all skins in the scene.
    pub fn all_skins(&self) -> impl Iterator<Item = &MainEntity> {
        self.skin_uniform_info.keys()
    }
}

/// Allocation information about each skin.
pub struct SkinUniformInfo {
    /// The allocation of the joints within the [`SkinUniforms::current_buffer`].
    pub allocation: Allocation,
    /// The entities that comprise the joints.
    pub joints: Vec<MainEntity>,
}

impl SkinUniformInfo {
    /// The offset in joints within the [`SkinUniforms::current_staging_buffer`].
    pub fn offset(&self) -> u32 {
        self.allocation.offset * JOINTS_PER_ALLOCATION_UNIT
    }
}
