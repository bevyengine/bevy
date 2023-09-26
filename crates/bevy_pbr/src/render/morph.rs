use std::mem;

use bevy_ecs::prelude::*;
use bevy_render::{
    batching::NoAutomaticBatching,
    mesh::morph::{MeshMorphWeights, MAX_MORPH_WEIGHTS},
    render_resource::{BufferUsages, BufferVec},
    renderer::{RenderDevice, RenderQueue},
    view::ViewVisibility,
    Extract,
};

#[derive(Component)]
pub struct MorphIndex {
    pub(super) index: u32,
}
#[derive(Resource)]
pub struct MorphUniform {
    pub buffer: BufferVec<f32>,
}
impl Default for MorphUniform {
    fn default() -> Self {
        Self {
            buffer: BufferVec::new(BufferUsages::UNIFORM),
        }
    }
}

pub fn prepare_morphs(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut uniform: ResMut<MorphUniform>,
) {
    if uniform.buffer.is_empty() {
        return;
    }
    let buffer = &mut uniform.buffer;
    buffer.reserve(buffer.len(), &device);
    buffer.write_buffer(&device, &queue);
}

pub fn extract_morphs(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut uniform: ResMut<MorphUniform>,
    query: Extract<Query<(Entity, &ViewVisibility, &MeshMorphWeights)>>,
) {
    uniform.buffer.clear();

    let mut values = Vec::with_capacity(*previous_len);

    for (entity, view_visibility, morph_weights) in &query {
        if !view_visibility.get() {
            continue;
        }
        let start = uniform.buffer.len();
        let weights = morph_weights.weights();
        let legal_weights = weights.iter().take(MAX_MORPH_WEIGHTS).copied();
        uniform.buffer.extend(legal_weights);
        uniform.buffer.add_to_alignment();

        let index = (start * mem::size_of::<f32>()) as u32;
        // NOTE: Because morph targets require per-morph target texture bindings, they cannot
        // currently be batched.
        values.push((entity, (MorphIndex { index }, NoAutomaticBatching)));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}
