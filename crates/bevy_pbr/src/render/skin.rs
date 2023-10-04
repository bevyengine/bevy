use bevy_asset::Assets;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_render::{
    batching::NoAutomaticBatching,
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    render_resource::{BufferUsages, BufferVec},
    renderer::{RenderDevice, RenderQueue},
    view::ViewVisibility,
    Extract,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::EntityHashMap;

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

#[derive(Default, Resource, Deref, DerefMut)]
pub struct SkinIndices(EntityHashMap<Entity, SkinIndex>);

// Notes on implementation: see comment on top of the `extract_skins` system.
#[derive(Resource)]
pub struct SkinUniform {
    pub buffer: BufferVec<Mat4>,
}

impl Default for SkinUniform {
    fn default() -> Self {
        Self {
            buffer: BufferVec::new(BufferUsages::UNIFORM),
        }
    }
}

pub fn prepare_skins(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniform: ResMut<SkinUniform>,
) {
    if uniform.buffer.is_empty() {
        return;
    }

    let len = uniform.buffer.len();
    uniform.buffer.reserve(len, &render_device);
    uniform.buffer.write_buffer(&render_device, &render_queue);
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
    mut skin_indices: ResMut<SkinIndices>,
    mut uniform: ResMut<SkinUniform>,
    query: Extract<Query<(Entity, &ViewVisibility, &SkinnedMesh)>>,
    inverse_bindposes: Extract<Res<Assets<SkinnedMeshInverseBindposes>>>,
    joints: Extract<Query<&GlobalTransform>>,
) {
    uniform.buffer.clear();
    skin_indices.clear();
    let mut last_start = 0;

    // PERF: This can be expensive, can we move this to prepare?
    for (entity, view_visibility, skin) in &query {
        if !view_visibility.get() {
            continue;
        }
        let buffer = &mut uniform.buffer;
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

        skin_indices.insert(entity, SkinIndex::new(start));
    }

    // Pad out the buffer to ensure that there's enough space for bindings
    while uniform.buffer.len() - last_start < MAX_JOINTS {
        uniform.buffer.push(Mat4::ZERO);
    }
}

// NOTE: The skinned joints uniform buffer has to be bound at a dynamic offset per
// entity and so cannot currently be batched.
pub fn no_automatic_skin_batching(
    mut commands: Commands,
    query: Query<Entity, (With<SkinnedMesh>, Without<NoAutomaticBatching>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(NoAutomaticBatching);
    }
}
