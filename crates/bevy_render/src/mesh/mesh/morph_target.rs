use super::Mesh;
use bevy_core::cast_slice;
use bevy_ecs::component::Component;
use std::ops::{Deref, DerefMut};

/// A [morph target] for a parent mesh. A given [`Mesh`] may have zero or more
/// morph targets that affect the final rendered result.
///
/// [morph target]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Debug, Clone)]
pub struct MorphTarget {
    pub position_displacement: Option<Vec<[f32; 3]>>,
    pub normal_displacement: Option<Vec<[f32; 3]>>,
    pub tangent_displacement: Option<Vec<[f32; 3]>>,
}

impl MorphTarget {
    const VERTEX_SIZE: usize = std::mem::size_of::<f32>() * 8;

    /// Counts all vertices of the morph target.
    ///
    /// # Panics
    /// Panics if the attributes have different vertex counts.
    pub fn count_vertices(&self) -> usize {
        let mut vertex_count = self.position_displacement.as_ref().map(|v| v.len());
        if let Some(ref attribute_data) = self.normal_displacement {
            let attribute_len = attribute_data.len();
            if let Some(previous_vertex_count) = vertex_count {
                assert_eq!(previous_vertex_count, attribute_len,
                        "Attribute {} has a different vertex count ({}) than other attributes ({}) in this morph target.", Mesh::ATTRIBUTE_NORMAL, attribute_len, previous_vertex_count);
            }
            vertex_count = Some(attribute_len);
        }
        if let Some(ref attribute_data) = self.tangent_displacement {
            let attribute_len = attribute_data.len();
            if let Some(previous_vertex_count) = vertex_count {
                assert_eq!(previous_vertex_count, attribute_len,
                        "Attribute {} has a different vertex count ({}) than other attributes ({}) in this morph target.", Mesh::ATTRIBUTE_TANGENT, attribute_len, previous_vertex_count);
            }
            vertex_count = Some(attribute_len);
        }

        vertex_count.unwrap_or(0)
    }

    pub fn get_vertex_buffer_data(&self) -> Vec<u8> {
        let vertex_count = self.count_vertices();
        let mut attributes_interleaved_buffer = vec![0; vertex_count * Self::VERTEX_SIZE];
        // bundle into interleaved buffers
        Self::interleave_bytes(
            &self.position_displacement,
            &mut attributes_interleaved_buffer,
            3,
            0,
        );
        Self::interleave_bytes(
            &self.normal_displacement,
            &mut attributes_interleaved_buffer,
            2,
            3,
        );
        Self::interleave_bytes(
            &self.tangent_displacement,
            &mut attributes_interleaved_buffer,
            3,
            5,
        );
        attributes_interleaved_buffer
    }

    fn interleave_bytes<T: bevy_core::Pod>(
        src: &Option<Vec<T>>,
        dst: &mut [u8],
        float_count: usize,
        float_offset: usize,
    ) {
        let attribute_offset = float_offset * std::mem::size_of::<f32>();
        let attribute_size = float_count * std::mem::size_of::<f32>();
        if let Some(attribute_values) = src.as_ref() {
            let attributes_bytes = cast_slice(attribute_values);
            for (vertex_index, attribute_bytes) in
                attributes_bytes.chunks_exact(attribute_size).enumerate()
            {
                let offset = vertex_index * Self::VERTEX_SIZE + attribute_offset;
                dst[offset..offset + attribute_size].copy_from_slice(attribute_bytes);
            }
        }
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
        let mut weights = [f32::MAX; N];
        let mut indexes = [0; N];
        let len = self.0.len();
        if N > len {
            for idx in 0..N {
                weights[idx] = self.0[idx];
                indexes[idx] = idx;
            }
        } else {
            for (idx, weight) in self.0.iter().cloned().enumerate() {
                let min_idx = Self::min_abs_idx(&weights);
                if weight.abs() > weights[min_idx].abs() {
                    indexes[min_idx] = idx;
                    weights[min_idx] = weight;
                }
            }
        };

        (weights, indexes)
    }

    #[inline(always)]
    fn min_abs_idx<const N: usize>(values: &[f32; N]) -> usize {
        let mut max_idx = 0;
        for idx in 0..N {
            if values[idx].abs() < values[max_idx].abs() {
                max_idx = idx;
            }
        }
        max_idx
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
