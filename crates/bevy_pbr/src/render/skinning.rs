use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_render::{
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    render_resource::BufferUsages,
    renderer::{RenderDevice, RenderQueue},
    view::ViewVisibility,
    Extract,
};
use bevy_transform::prelude::GlobalTransform;

use super::double_buffer::DoubleBufferVec;

/// Maximum number of joints supported for skinned meshes.
pub const MAX_JOINTS: usize = 256;
const JOINT_SIZE: usize = std::mem::size_of::<Mat4>();
pub(crate) const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

#[derive(Component)]
pub struct SkinIndex {
    pub index: u32,
}
#[derive(Resource)]
pub struct SkinUniform {
    pub buffer: DoubleBufferVec<Mat4>,
}
impl Default for SkinUniform {
    fn default() -> Self {
        Self {
            buffer: DoubleBufferVec::new(BufferUsages::UNIFORM),
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

impl SkinIndex {
    /// Updated index to be in address space based on [`SkinnedMeshUniform`] size.
    pub fn to_buffer_index(mut self) -> Self {
        self.index *= std::mem::size_of::<Mat4>() as u32;
        self
    }
}

pub fn extract_skins(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut uniform: ResMut<SkinUniform>,
    query: Extract<Query<(Entity, &ViewVisibility, &SkinnedMesh)>>,
    inverse_bindposes: Extract<Res<Assets<SkinnedMeshInverseBindposes>>>,
    joints: Extract<Query<&GlobalTransform>>,
) {
    uniform.buffer.clear();

    let mut values = Vec::with_capacity(*previous_len);
    let mut last_start = 0;

    for (entity, view_visibility, skin) in &query {
        if !view_visibility.get() {
            continue;
        }
        // PERF: This can be expensive, can we move this to prepare?
        let buffer = uniform.buffer.current_buffer_mut();
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

        // Pad to 256 byte alignment
        while buffer.len() % 4 != 0 {
            buffer.push(Mat4::ZERO);
        }
        let index = start as u32;
        let skin_index = SkinIndex { index };
        last_start = last_start.max(skin_index.index as usize);
        values.push((entity, skin_index.to_buffer_index()));
    }

    // Pad out the buffer to ensure that there's enough space for bindings
    while uniform.buffer.len() - last_start < MAX_JOINTS {
        uniform.buffer.current_buffer_mut().push(Mat4::ZERO);
    }

    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}
