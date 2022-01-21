use super::Mesh;
use bevy_ecs::component::Component;
use bevy_math::Vec3;
use std::ops::{Deref, DerefMut};

/// A [morph target] for a parent mesh. A given [`Mesh`] may have zero or more
/// morph targets that affect the final rendered result.
///
/// [morph target]: https://en.wikipedia.org/wiki/Morph_target_animation
#[derive(Debug, Clone)]
pub struct MorphTarget {
    pub position_displacement: Option<Vec<Vec3>>,
    pub normal_displacement: Option<Vec<Vec3>>,
    pub tangent_displacement: Option<Vec<Vec3>>,
}

impl MorphTarget {
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
