use bevy_asset::Assets;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_render::{
    batching::NoAutomaticBatching,
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    render_resource::{Buffer, BufferUsages, BufferVec, StorageBuffer},
    renderer::{RenderDevice, RenderQueue},
    view::ViewVisibility,
    Extract,
};
use bevy_transform::prelude::GlobalTransform;

/// Maximum number of joints supported for skinned meshes.
pub const MAX_JOINTS: usize = 256;

#[derive(Component)]
pub enum SkinIndex {
    Index(u32),
    DynamicOffset(u32),
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct SkinIndices(EntityHashMap<SkinIndex>);

// Notes on implementation: see comment on top of the `extract_skins` system.
#[derive(Resource)]
pub enum SkinUniform {
    Uniform(BufferVec<Mat4>),
    Storage(StorageBuffer<Vec<Mat4>>),
}

impl SkinUniform {
    pub fn new(render_device: &RenderDevice) -> Self {
        if render_device.limits().max_storage_buffers_per_shader_stage > 0 {
            Self::Storage(StorageBuffer::<Vec<Mat4>>::default())
        } else {
            Self::Uniform(BufferVec::new(BufferUsages::UNIFORM))
        }
    }

    pub fn len(&self) -> usize {
        match self {
            SkinUniform::Uniform(buffer) => buffer.len(),
            SkinUniform::Storage(buffer) => buffer.get().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            SkinUniform::Uniform(buffer) => buffer.is_empty(),
            SkinUniform::Storage(buffer) => buffer.get().is_empty(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            SkinUniform::Uniform(buffer) => buffer.clear(),
            SkinUniform::Storage(buffer) => buffer.get_mut().clear(),
        }
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        match self {
            SkinUniform::Uniform(buffer) => buffer.write_buffer(device, queue),
            SkinUniform::Storage(buffer) => buffer.write_buffer(device, queue),
        }
    }

    pub fn push(&mut self, value: Mat4) {
        match self {
            SkinUniform::Uniform(buffer) => {
                buffer.push(value);
            }
            SkinUniform::Storage(buffer) => buffer.get_mut().push(value),
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = Mat4>) {
        match self {
            SkinUniform::Uniform(buffer) => buffer.extend(iter),
            SkinUniform::Storage(buffer) => buffer.get_mut().extend(iter),
        }
    }

    pub fn truncate(&mut self, len: usize) {
        match self {
            SkinUniform::Uniform(buffer) => buffer.truncate(len),
            SkinUniform::Storage(buffer) => buffer.get_mut().truncate(len),
        }
    }

    pub fn align_to_dynamic_offset(&mut self) {
        match self {
            SkinUniform::Uniform(buffer) => {
                // Pad to 256 byte alignment
                while buffer.len() % 4 != 0 {
                    buffer.push(Mat4::ZERO);
                }
            }
            SkinUniform::Storage(_) => {}
        }
    }

    pub fn pad_to_len(&mut self, len: usize) {
        match self {
            SkinUniform::Uniform(buffer) => {
                // Pad out the buffer to ensure that there's enough space for the last binding
                while buffer.len() < len {
                    buffer.push(Mat4::ZERO);
                }
            }
            SkinUniform::Storage(_) => {}
        }
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        match self {
            SkinUniform::Uniform(buffer) => buffer.buffer(),
            SkinUniform::Storage(buffer) => buffer.buffer(),
        }
    }

    pub fn index(&self) -> SkinIndex {
        match self {
            SkinUniform::Uniform(buffer) => {
                SkinIndex::DynamicOffset((buffer.len() * std::mem::size_of::<Mat4>()) as u32)
            }
            SkinUniform::Storage(buffer) => SkinIndex::Index(buffer.get().len() as u32),
        }
    }
}

pub fn prepare_skins(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniform: ResMut<SkinUniform>,
) {
    if uniform.is_empty() {
        return;
    }

    uniform.write_buffer(&render_device, &render_queue);
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
    uniform.clear();
    skin_indices.clear();
    let mut last_start = 0;

    // PERF: This can be expensive, can we move this to prepare?
    for (entity, view_visibility, skin) in &query {
        if !view_visibility.get() {
            continue;
        }
        let Some(inverse_bindposes) = inverse_bindposes.get(&skin.inverse_bindposes) else {
            continue;
        };
        let start = uniform.len();
        let skin_index = uniform.index();

        let target = start + skin.joints.len().min(MAX_JOINTS);
        uniform.extend(
            joints
                .iter_many(&skin.joints)
                .zip(inverse_bindposes.iter())
                .take(MAX_JOINTS)
                .map(|(joint, bindpose)| joint.affine() * *bindpose),
        );
        // iter_many will skip any failed fetches. This will cause it to assign the wrong bones,
        // so just bail by truncating to the start.
        if uniform.len() != target {
            uniform.truncate(start);
            continue;
        }
        last_start = last_start.max(start);

        uniform.align_to_dynamic_offset();

        skin_indices.insert(entity, skin_index);
    }

    uniform.pad_to_len(last_start + MAX_JOINTS);
}

// NOTE: If skinned joints is a uniform buffer, it has to be bound at a dynamic offset per
// entity and so cannot currently be batched.
pub fn no_automatic_skin_batching(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    query: Query<Entity, (With<SkinnedMesh>, Without<NoAutomaticBatching>)>,
) {
    if render_device.limits().max_storage_buffers_per_shader_stage > 0 {
        // SkinUniform is using a storage buffer so skinned meshes can be batched
        return;
    }

    for entity in &query {
        commands.entity(entity).try_insert(NoAutomaticBatching);
    }
}
