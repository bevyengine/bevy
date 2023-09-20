use std::mem;

use bevy_render::{
    render_resource::{Buffer, BufferUsages, BufferVec},
    renderer::{RenderDevice, RenderQueue},
};
use bytemuck::Pod;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    A,
    B,
}
impl Side {
    fn toggle(&mut self) {
        *self = match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}
pub struct DoubleBufferVec<T: Pod> {
    a: BufferVec<T>,
    b: BufferVec<T>,
    current: Side,
}
impl<T: Pod> Extend<T> for DoubleBufferVec<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.current_buffer_mut().extend(iter);
    }
}
impl<T: Pod> DoubleBufferVec<T> {
    pub const fn new(buffer_usage: BufferUsages) -> Self {
        DoubleBufferVec {
            a: BufferVec::new(buffer_usage),
            b: BufferVec::new(buffer_usage),
            current: Side::A,
        }
    }
    pub(crate) const fn current_buffer(&self) -> &BufferVec<T> {
        match self.current {
            Side::A => &self.a,
            Side::B => &self.b,
        }
    }
    pub(crate) fn current_buffer_mut(&mut self) -> &mut BufferVec<T> {
        match self.current {
            Side::A => &mut self.a,
            Side::B => &mut self.b,
        }
    }
    pub fn buffer(&self) -> Option<&Buffer> {
        self.current_buffer().buffer()
    }
    pub fn old_buffer(&self) -> Option<&Buffer> {
        let old_buffer = match self.current {
            Side::A => &self.b,
            Side::B => &self.a,
        };
        old_buffer.buffer()
    }
    pub fn is_empty(&self) -> bool {
        self.current_buffer().is_empty()
    }
    pub fn len(&self) -> usize {
        self.current_buffer().len()
    }
    pub fn clear(&mut self) {
        self.current.toggle();
        self.current_buffer_mut().clear();
    }
    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        self.current_buffer_mut().reserve(capacity, device);
    }
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.current_buffer_mut().write_buffer(device, queue);
    }
}
