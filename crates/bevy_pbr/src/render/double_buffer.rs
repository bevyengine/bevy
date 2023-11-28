use std::mem;

use bevy_ecs::prelude::Entity;
use bevy_render::render_resource::{BufferUsages, BufferVec};
use bevy_utils::EntityHashMap;
use bytemuck::Pod;

/// A double buffer of `T`.
///
/// Use [`DoubleBuffer::swap`] to swap buffer,
/// access the current buffer with [`DoubleBuffer::current`],
/// and the previous one with [`DoubleBuffer::previous`].
#[derive(Default)]
pub struct DoubleBuffer<T> {
    pub previous: T,
    pub current: T,
}

impl<T> DoubleBuffer<T> {
    pub fn swap(&mut self, swap_buffer: bool) {
        if swap_buffer {
            mem::swap(&mut self.current, &mut self.previous);
        }
    }
}

pub type DoubleBufferVec<T> = DoubleBuffer<BufferVec<T>>;

impl<T: Pod> DoubleBufferVec<T> {
    pub const fn new(buffer_usage: BufferUsages) -> Self {
        DoubleBufferVec {
            previous: BufferVec::new(buffer_usage),
            current: BufferVec::new(buffer_usage),
        }
    }

    pub fn clear(&mut self, swap_buffer: bool) {
        self.swap(swap_buffer);
        self.current.clear();
    }
}

pub type DoubleEntityMap<T> = DoubleBuffer<EntityHashMap<Entity, T>>;

impl<T> DoubleEntityMap<T> {
    pub fn clear(&mut self, swap_buffer: bool) {
        self.swap(swap_buffer);
        self.current.clear();
    }

    pub fn insert(&mut self, entity: Entity, value: T) {
        self.current.insert(entity, value);
    }

    pub fn missing_previous(&self, entity: &Entity) -> bool {
        let current = self.current.contains_key(entity);
        let previous = self.previous.contains_key(entity);
        // Either it's already missing (therefore there is no "previous" to miss)
        // or it's not missing and there is no "previous", so we miss previous.
        current && !previous
    }
}
