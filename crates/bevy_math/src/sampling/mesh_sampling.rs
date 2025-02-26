//! Functionality related to random sampling from triangle meshes.

use crate::{
    primitives::{Measured2d, Triangle3d},
    ShapeSample, Vec3,
};
use alloc::vec::Vec;
use rand::Rng;
use rand_distr::{
    weighted::{Error as WeightedError, WeightedAliasIndex},
    Distribution,
};

/// A [distribution] that caches data to allow fast sampling from a collection of triangles.
/// Generally used through [`sample`] or [`sample_iter`].
///
/// [distribution]: Distribution
/// [`sample`]: Distribution::sample
/// [`sample_iter`]: Distribution::sample_iter
///
/// Example
/// ```
/// # use bevy_math::{Vec3, primitives::*};
/// # use bevy_math::sampling::mesh_sampling::UniformMeshSampler;
/// # use rand::{SeedableRng, rngs::StdRng, distr::Distribution};
/// let faces = Tetrahedron::default().faces();
/// let sampler = UniformMeshSampler::try_new(faces).unwrap();
/// let rng = StdRng::seed_from_u64(8765309);
/// // 50 random points on the tetrahedron:
/// let samples: Vec<Vec3> = sampler.sample_iter(rng).take(50).collect();
/// ```
pub struct UniformMeshSampler {
    triangles: Vec<Triangle3d>,
    face_distribution: WeightedAliasIndex<f32>,
}

impl Distribution<Vec3> for UniformMeshSampler {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let face_index = self.face_distribution.sample(rng);
        self.triangles[face_index].sample_interior(rng)
    }
}

impl UniformMeshSampler {
    /// Construct a new [`UniformMeshSampler`] from a list of [triangles].
    ///
    /// Returns an error if the distribution of areas for the collection of triangles could not be formed
    /// (most notably if the collection has zero surface area).
    ///
    /// [triangles]: Triangle3d
    pub fn try_new<T: IntoIterator<Item = Triangle3d>>(
        triangles: T,
    ) -> Result<Self, WeightedError> {
        let triangles: Vec<Triangle3d> = triangles.into_iter().collect();
        let areas = triangles.iter().map(Measured2d::area).collect();

        WeightedAliasIndex::new(areas).map(|face_distribution| Self {
            triangles,
            face_distribution,
        })
    }
}
