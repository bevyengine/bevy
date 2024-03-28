use crate::meshes::TriangleMesh;
use crate::{ShapeSample, Vec3};
use rand::Rng;
use rand_distr::{Distribution, WeightedAliasIndex};
use thiserror::Error;

/// A wrapper that caches data to allow fast sampling from the surface of a mesh. Generally used via
/// [`Distribution::sample`] or [`Distribution::sample_iter`].
///
/// Example
/// ```
/// # use bevy_math::{Vec3, UniformMeshSampler, primitives::Torus};
/// # use bevy_render::prelude::Meshable;
/// # use rand::{SeedableRng, rngs::StdRng, distributions::Distribution};
/// let my_torus_mesh = Torus::new(1.0, 0.5).mesh().build();
/// let torus_sampler = UniformMeshSampler::try_new(&my_torus_mesh).unwrap();
/// let rng = StdRng::from_entropy();
/// // 50 random points on the torus:
/// let torus_samples: Vec<Vec3> = torus_sampler.sample_iter(rng).take(50).collect();
/// ```
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

/// An error that indicates that we were unable to form a [`UniformMeshSampler`] from the given data,
/// either because its conversion to a [`TriangleMesh`] failed or because the resulting [`TriangleMesh`]
/// had zero area.
#[derive(Debug, Error)]
pub enum MeshSamplerConstructionError<TriangleMeshingError> {
    /// A [`TriangleMesh`] was successfully constructed, but it had zero area, so we were unable to create
    /// a distribution for it.
    DistributionFormation(ZeroAreaMeshError),

    /// A [`TriangleMesh`] could not be constructed.
    TriangleMeshing {
        #[from]
        /// The underlying reason for the failure in construction of the [`TriangleMesh`]
        error: TriangleMeshingError,
    },
}

impl UniformMeshSampler {
    /// Construct a new [`UniformMeshSampler`] from anything that can be fallibly converted into a [`TriangleMesh`].
    ///
    /// Returns an error if the intermediate [`TriangleMesh`] conversion fails or if it has zero surface area.
    pub fn try_new<T: TryInto<TriangleMesh>>(
        meshable: T,
    ) -> Result<Self, MeshSamplerConstructionError<T::Error>> {
        let tri_mesh: TriangleMesh = meshable.try_into()?;
        tri_mesh
            .try_into()
            .map_err(MeshSamplerConstructionError::DistributionFormation)
    }
}
