use crate::{
    render_resource::{std140::AsStd140, std430::AsStd430, Buffer, BufferUsages, BufferVec, DynamicUniformVec},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::component::Component;
use bevy_math::Vec3;
use bevy_core::{Zeroable, Pod};
use std::{
    cmp::min,
    ops::{Deref, DerefMut, Range},
};

#[derive(Debug, Default, Clone, Copy, AsStd430, Zeroable, Pod)]
#[repr(C)]
pub struct MorphTargetDisplacement {
    pub index: u32,
    pub position: Vec3,
    pub normal: Vec3,
    pub tangent: Vec3,
}

#[derive(Debug, Clone, Copy, AsStd140, Zeroable, Pod)]
#[repr(C)]
pub struct MorphTargetUniform {
    pub start: u32,
    pub count: u32,
}

#[derive(Debug, Default, Clone, Copy, AsStd430, Zeroable, Pod)]
#[repr(C)]
struct FinalDisplacement {
    position: Vec3,
    normal: Vec3,
    tangent: Vec3,
}

/// A [morph target] for a parent mesh. A given [`Mesh`] may have zero or more
/// morph targets that affect the final rendered result.
///
/// [morph target]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Debug, Default, Clone)]
pub struct MorphTargets {
    displacements: Vec<MorphTargetDisplacement>,
    ranges: Vec<Range<usize>>,
}

impl MorphTargets {
    /// Gets the number of morph targets stored within.
    #[inline]
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Checks if the morph target set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn add_target(
        &mut self,
        positions: Option<Vec<[f32; 3]>>,
        normals: Option<Vec<[f32; 3]>>,
        tangents: Option<Vec<[f32; 3]>>,
    ) {
        const ZERO: [f32; 3] = [0.0, 0.0, 0.0];

        let positions = positions.unwrap_or_else(Vec::new);
        let normals = normals.unwrap_or_else(Vec::new);
        let tangents = tangents.unwrap_or_else(Vec::new);
        let len = positions.len().max(normals.len().max(tangents.len()));

        let start = self.displacements.len();
        for index in 0..len {
            let position = positions.get(index).copied().unwrap_or(ZERO);
            let normal = normals.get(index).copied().unwrap_or(ZERO);
            let tangent = tangents.get(index).copied().unwrap_or(ZERO);
            if position != ZERO || normal != ZERO || tangent != ZERO {
                self.displacements.push(MorphTargetDisplacement {
                    index: index
                        .try_into()
                        .expect("Attempted to load mesh with more than u32::MAX vertices."),
                    position: position.into(),
                    normal: position.into(),
                    tangent: position.into(),
                });
            }
        }

        self.ranges.push(start..self.displacements.len());
    }

    pub(crate) fn build_displacement_buffer(&self, render_device: &RenderDevice) -> Buffer {
        let mut buffer_vec = BufferVec::new(BufferUsages::STORAGE);
        buffer_vec.reserve(self.displacements.len(), render_device);
        for displacement in self.displacements.iter() {
            buffer_vec.push(displacement.clone());
        }
        buffer_vec.write(render_device, self.displacements.len());
        buffer_vec.buffer().unwrap().clone()
    }

    pub(crate) fn build_range_buffer(&self, render_device: &RenderDevice) -> Buffer {
        let mut buffer_vec = DynamicUniformVec::new();
        buffer_vec.reserve(self.ranges.len(), render_device);
        for range in self.ranges.iter() {
            buffer_vec.push(MorphTargetUniform {
                start: range.start as u32,
                count: (range.end - range.start) as u32,
            });
        }
        buffer_vec.write(render_device, self.ranges.len());
        buffer_vec.uniform_buffer().unwrap().clone()
    }

    pub(crate) fn build_final_displacement_buffer(
        &self,
        render_device: &RenderDevice,
        vertex_count: usize,
    ) -> Buffer {
        let mut buffer_vec = BufferVec::new(BufferUsages::STORAGE);
        buffer_vec.reserve(render_device, vertex_count);
        for range in 0..vertex_count {
            buffer_vec.push(FinalDisplacement::default());
        }
        buffer_vec.write(render_device, self.ranges.len());
        buffer_vec.buffer().unwrap().clone()
    }
}

/// A [`Component`] for storing the live weights of a [`Mesh`]'s morph targets.
#[derive(Debug, Clone, Component)]
pub struct MorphTargetWeights(Vec<f32>);

impl MorphTargetWeights {
    /// Gets the weight and indexes with the highest absolute value among the
    /// weights. If the number of weights are present is lower than `N`, the
    /// remainder will be filled with zero-values.
    ///
    /// The returned values are returned in non particular order.
    pub fn strongest_n<const N: usize>(&self) -> ([f32; N], [usize; N]) {
        let mut weights = [0.0; N];
        let mut indexes = [0; N];
        let len = self.0.len();
        for idx in 0..min(len, N) {
            weights[idx] = self.0[idx];
            indexes[idx] = idx;
        }
        if N < len {
            let mut min_idx = Self::min_abs_idx(&weights);
            let mut min = weights[min_idx];
            for (idx, weight) in self.0.iter().cloned().enumerate().skip(N) {
                if weight.abs() > min {
                    weights[min_idx] = weight;
                    indexes[min_idx] = idx;
                    min_idx = Self::min_abs_idx(&weights);
                    min = weights[min_idx];
                }
            }
        }

        (weights, indexes)
    }

    #[inline(always)]
    fn min_abs_idx<const N: usize>(values: &[f32; N]) -> usize {
        let mut min = f32::MAX;
        let mut min_idx = 0;
        for (idx, value) in values.iter().cloned().map(f32::abs).enumerate() {
            if value < min {
                min = value;
                min_idx = idx;
            }
        }
        min_idx
    }
}

impl Deref for MorphTargetWeights {
    type Target = Vec<f32>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MorphTargetWeights {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
