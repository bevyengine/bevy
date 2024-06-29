//! Functionality related to random sampling from triangle meshes.

use crate::{primitives::Measured2d, primitives::Triangle3d, ShapeSample, Vec3};
use rand::Rng;
use rand_distr::{Distribution, WeightedAliasIndex};
use thiserror::Error;

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
/// # use rand::{SeedableRng, rngs::StdRng, distributions::Distribution};
/// let faces: Vec<Triangle3d> = Tetrahedron::default().faces().into();
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

/// An error that indicates that a [`UniformMeshSampler`] could not be constructed
/// because its input data had a total area of zero.
#[derive(Debug, Error)]
#[error("Failed to form distribution: provided triangles have zero area")]
pub struct ZeroAreaMeshError;

impl TryFrom<Vec<Triangle3d>> for UniformMeshSampler {
    type Error = ZeroAreaMeshError;

    fn try_from(triangles: Vec<Triangle3d>) -> Result<Self, Self::Error> {
        let areas = triangles.iter().map(|t| t.area()).collect();
        let Ok(face_distribution) = WeightedAliasIndex::new(areas) else {
            return Err(ZeroAreaMeshError);
        };

        Ok(Self {
            triangles,
            face_distribution,
        })
    }
}

impl UniformMeshSampler {
    /// Construct a new [`UniformMeshSampler`] from a list of triangles.
    ///
    /// Returns an error if the collection of triangles would have zero surface area.
    pub fn try_new<T: Into<Vec<Triangle3d>>>(triangles: T) -> Result<Self, ZeroAreaMeshError> {
        let triangles: Vec<Triangle3d> = triangles.into();
        triangles.try_into()
    }
}
