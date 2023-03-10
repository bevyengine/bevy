use std::{iter, mem};

use bevy_ecs::prelude::*;
use bevy_render::{
    mesh::morph::{MorphWeights, MAX_MORPH_WEIGHTS},
    render_resource::{BufferUsages, BufferVec},
    renderer::{RenderDevice, RenderQueue},
    view::ComputedVisibility,
    Extract,
};
use bytemuck::Pod;

#[derive(Component)]
pub struct Index {
    pub(super) index: u32,
}
#[derive(Resource)]
pub struct Uniform {
    pub buffer: BufferVec<f32>,
}
impl Default for Uniform {
    fn default() -> Self {
        Self {
            buffer: BufferVec::new(BufferUsages::UNIFORM),
        }
    }
}

pub fn prepare(device: Res<RenderDevice>, queue: Res<RenderQueue>, mut uniform: ResMut<Uniform>) {
    if uniform.buffer.is_empty() {
        return;
    }
    let buffer = &mut uniform.buffer;
    buffer.reserve(buffer.len(), &device);
    buffer.write_buffer(&device, &queue);
}

const fn can_align(step: usize, target: usize) -> bool {
    step % target == 0 || target % step == 0
}

/// Align a [`BufferVec`] to `N` bytes by padding the end with `T::default()` values.
fn add_to_alignment<const N: usize, T: Pod + Default>(buffer: &mut BufferVec<T>) {
    let t_size = mem::size_of::<T>();
    if !can_align(t_size, N) {
        panic!("BufferVec should contain only types with a size multiple or divisible by N");
    }
    let buffer_size = buffer.len();
    let byte_size = t_size * buffer_size;
    let bytes_over_n = byte_size % N;
    if bytes_over_n == 0 {
        return;
    }
    let bytes_to_add = N - bytes_over_n;
    let ts_to_add = bytes_to_add / t_size;
    buffer.extend(iter::repeat_with(T::default).take(ts_to_add));
}

pub fn extract(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut uniform: ResMut<Uniform>,
    query: Extract<Query<(Entity, &ComputedVisibility, &MorphWeights)>>,
) {
    uniform.buffer.clear();

    let mut values = Vec::with_capacity(*previous_len);

    for (entity, computed_visibility, morph_weights) in &query {
        if !computed_visibility.is_visible() {
            continue;
        }
        let start = uniform.buffer.len();
        let weights = morph_weights.weights();
        let legal_weights = weights.iter().take(MAX_MORPH_WEIGHTS).copied();
        uniform.buffer.extend(legal_weights);
        add_to_alignment::<256, f32>(&mut uniform.buffer);

        let index = (start * mem::size_of::<f32>()) as u32;
        values.push((entity, Index { index }));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}
