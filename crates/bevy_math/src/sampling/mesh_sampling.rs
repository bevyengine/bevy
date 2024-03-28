use crate::meshes::TriangleMesh;
use crate::{ShapeSample, Vec3};
use rand::Rng;
use rand_distr::{Distribution, WeightedAliasIndex};
use thiserror::Error;

/// A wrapper struct that caches data to allow fast sampling from the surface
/// of a mesh. Used via [`Distribution::sample`].
pub struct UniformMeshSampler {
    triangle_mesh: TriangleMesh,
    face_distribution: WeightedAliasIndex<f32>,
}

impl Distribution<Vec3> for UniformMeshSampler {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let face_index = self.face_distribution.sample(rng);
        self.triangle_mesh.faces[face_index].sample_interior(rng)
    }
}

/// An error that indicates that a [`UniformMeshSampler`] could not be constructed
/// because its input data had a total area of zero.
#[derive(Debug, Error)]
#[error("Failed to form distribution: provided mesh has zero area")]
pub struct ZeroAreaMeshError;

impl TryFrom<TriangleMesh> for UniformMeshSampler {
    type Error = ZeroAreaMeshError;

    fn try_from(triangle_mesh: TriangleMesh) -> Result<Self, Self::Error> {
        let areas = triangle_mesh.faces.iter().map(|t| t.area()).collect();
        let Ok(face_distribution) = WeightedAliasIndex::new(areas) else {
            return Err(ZeroAreaMeshError);
        };

        Ok(Self {
            triangle_mesh,
            face_distribution,
        })
    }
}
