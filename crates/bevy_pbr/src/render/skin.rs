use std::mem;

use bevy_asset::Assets;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_render::{
    batching::NoAutomaticBatching,
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    render_resource::{BufferUsages, RawBufferVec},
    renderer::{RenderDevice, RenderQueue},
    view::ViewVisibility,
    Extract,
};
use bevy_transform::prelude::GlobalTransform;

/// Maximum number of joints supported for skinned meshes.
pub const MAX_JOINTS: usize = 256;

#[derive(Component)]
pub struct SkinIndex {
    pub index: u32,
}

impl SkinIndex {
    /// Index to be in address space based on [`SkinUniform`] size.
    const fn new(start: usize) -> Self {
        SkinIndex {
            index: (start * std::mem::size_of::<Mat4>()) as u32,
        }
    }
}

/// Maps each skinned mesh to the applicable offset within the [`SkinUniforms`]
/// buffer.
///
/// We store both the current frame's joint matrices and the previous frame's
/// joint matrices for the purposes of motion vector calculation.
#[derive(Default, Resource)]
pub struct SkinIndices {
    /// Maps each skinned mesh to the applicable offset within
    /// [`SkinUniforms::current_buffer`].
    pub current: EntityHashMap<SkinIndex>,

    /// Maps each skinned mesh to the applicable offset within
    /// [`SkinUniforms::prev_buffer`].
    pub prev: EntityHashMap<SkinIndex>,
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
    /// Stores all the joint matrices for skinned meshes in the current frame.
    pub current_buffer: RawBufferVec<Mat4>,
    /// Stores all the joint matrices for skinned meshes in the previous frame.
    pub prev_buffer: RawBufferVec<Mat4>,
}

impl Default for SkinUniforms {
    fn default() -> Self {
        Self {
            current_buffer: RawBufferVec::new(BufferUsages::UNIFORM),
            prev_buffer: RawBufferVec::new(BufferUsages::UNIFORM),
        }
    }
}

pub fn prepare_skins(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniform: ResMut<SkinUniforms>,
) {
    if uniform.current_buffer.is_empty() {
        return;
    }

    let len = uniform.current_buffer.len();
    uniform.current_buffer.reserve(len, &render_device);
    uniform
        .current_buffer
        .write_buffer(&render_device, &render_queue);

    // We don't need to write `uniform.prev_buffer` because we already wrote it
    // last frame, and the data should still be on the GPU.
}

// Notes on implementation:
// We define the uniform binding as an array<mat4x4<f32>, N> in the shader,
// where N is the maximum number of Mat4s we can fit in the uniform binding,
// which may be as little as 16kB or 64kB. But, we may not need all N.
// We may only need, for example, 10.
//
// If we used uniform buffers ‘normally’ then we would have to write a full
// binding of data for each dynamic offset binding, which is wasteful, makes
// the buffer much larger than it needs to be, and uses more memory bandwidth
// to transfer the data, which then costs frame time So @superdump came up
// with this design: just bind data at the specified offset and interpret
// the data at that offset as an array<T, N> regardless of what is there.
//
// So instead of writing N Mat4s when you only need 10, you write 10, and
// then pad up to the next dynamic offset alignment. Then write the next.
// And for the last dynamic offset binding, make sure there is a full binding
// of data after it so that the buffer is of size
// `last dynamic offset` + `array<mat4x4<f32>>`.
//
// Then when binding the first dynamic offset, the first 10 entries in the array
// are what you expect, but if you read the 11th you’re reading ‘invalid’ data
// which could be padding or could be from the next binding.
//
// In this way, we can pack ‘variable sized arrays’ into uniform buffer bindings
// which normally only support fixed size arrays. You just have to make sure
// in the shader that you only read the values that are valid for that binding.
pub fn extract_skins(
    skin_indices: ResMut<SkinIndices>,
    uniform: ResMut<SkinUniforms>,
    query: Extract<Query<(Entity, &ViewVisibility, &SkinnedMesh)>>,
    inverse_bindposes: Extract<Res<Assets<SkinnedMeshInverseBindposes>>>,
    joints: Extract<Query<&GlobalTransform>>,
) {
    // Borrow check workaround.
    let (skin_indices, uniform) = (skin_indices.into_inner(), uniform.into_inner());

    // Swap buffers. We need to keep the previous frame's buffer around for the
    // purposes of motion vector computation.
    mem::swap(&mut skin_indices.current, &mut skin_indices.prev);
    mem::swap(&mut uniform.current_buffer, &mut uniform.prev_buffer);
    skin_indices.current.clear();
    uniform.current_buffer.clear();

    let mut last_start = 0;

    // PERF: This can be expensive, can we move this to prepare?
    for (entity, view_visibility, skin) in &query {
        if !view_visibility.get() {
            continue;
        }
        let buffer = &mut uniform.current_buffer;
        let Some(inverse_bindposes) = inverse_bindposes.get(&skin.inverse_bindposes) else {
            continue;
        };
        let start = buffer.len();

        let target = start + skin.joints.len().min(MAX_JOINTS);
        buffer.extend(
            joints
                .iter_many(&skin.joints)
                .zip(inverse_bindposes.iter())
                .take(MAX_JOINTS)
                .map(|(joint, bindpose)| joint.affine() * *bindpose),
        );
        // iter_many will skip any failed fetches. This will cause it to assign the wrong bones,
        // so just bail by truncating to the start.
        if buffer.len() != target {
            buffer.truncate(start);
            continue;
        }
        last_start = last_start.max(start);

        // Pad to 256 byte alignment
        while buffer.len() % 4 != 0 {
            buffer.push(Mat4::ZERO);
        }

        skin_indices.current.insert(entity, SkinIndex::new(start));
    }

    // Pad out the buffer to ensure that there's enough space for bindings
    while uniform.current_buffer.len() - last_start < MAX_JOINTS {
        uniform.current_buffer.push(Mat4::ZERO);
    }
}

// NOTE: The skinned joints uniform buffer has to be bound at a dynamic offset per
// entity and so cannot currently be batched.
pub fn no_automatic_skin_batching(
    mut commands: Commands,
    query: Query<Entity, (With<SkinnedMesh>, Without<NoAutomaticBatching>)>,
) {
    for entity in &query {
        commands.entity(entity).try_insert(NoAutomaticBatching);
    }
}
