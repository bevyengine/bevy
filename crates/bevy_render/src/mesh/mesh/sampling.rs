use crate::{mesh::Indices, Mesh};
use bevy_math::{primitives::Triangle3d, Vec3, ShapeSample};
use rand::Rng;
use rand_distr::{Distribution, WeightedAliasIndex};
use thiserror::Error;

/// A wrapper struct that caches data to allow O(1) sampling from the surface
/// of a mesh. Used via [`Distribution::sample`].
pub struct MeshSampler {
    faces: Vec<Triangle3d>,
    face_distribution: WeightedAliasIndex<f32>,
}

pub struct DistributionError;

impl MeshSampler {
    pub fn from_face_mesh(mesh: IndexedFaceMesh) -> Result<Self, DistributionError> {
        let faces = mesh.face_triangles();
        let areas = faces.iter().map(|t| t.area()).collect();
        let Ok(face_distribution) = WeightedAliasIndex::new(areas) else {
            return Err(DistributionError);
        };

        Ok(Self {
            faces,
            face_distribution,
        })
    }
}

impl Distribution<Vec3> for MeshSampler {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let face_index = self.face_distribution.sample(rng);
        self.faces[face_index].sample_interior(rng)
    }
}

pub struct IndexedFaceMesh {
    vertices: Vec<Vec3>,
    faces: Vec<[usize; 3]>,
}

#[derive(Debug, Error)]
pub enum InvalidMeshError {
    #[error("Source mesh lacks position data")]
    MissingPositions,

    #[error("Source mesh position format is not convertible to Vec3")]
    PositionsFormat,

    #[error("Source mesh lacks face index data")]
    MissingIndices,

    #[error("Index count {count} is not a multiple of 3")]
    IndexCount {
        count: usize
    },
}

impl IndexedFaceMesh {
    /// Build an [`IndexedFaceMesh`] from a [`Mesh`].
    ///
    /// This process is both extremely lossy and fallible.
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, InvalidMeshError> {
        let Some(position_data) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            return Err(InvalidMeshError::MissingPositions);
        };
        let Some(positions) = position_data.as_float3() else {
            return Err(InvalidMeshError::PositionsFormat);
        };
        let vertices: Vec<Vec3> = positions.iter().map(|pos| (*pos).into()).collect();

        let Some(indices) = mesh.indices() else {
            return Err(InvalidMeshError::MissingIndices);
        };

        // If the indices doesn't have a length divisible by 3, then this
        // fails with an error; this is `None` bubbling up from `take_three_u16`
        // or `take_three_u32`.
        let Some(faces): Option<Vec<[usize; 3]>> = (match indices {
            Indices::U16(vec) => vec.as_slice().chunks(3).map(take_three_u16).collect(),
            Indices::U32(vec) => vec.as_slice().chunks(3).map(take_three_u32).collect(),
        }) else {
            return Err(InvalidMeshError::IndexCount { count: indices.len() });
        };

        Ok(Self {
            vertices,
            faces,
        })
    }

    /// Build a face from the indices of its vertices.
    #[inline]
    fn build_face_triangle(vertices: &[Vec3], indices: [usize; 3]) -> Triangle3d {
        let vertices = indices.map(|v| vertices[v as usize]);
        Triangle3d { vertices }
    }

    pub fn face_triangle(&self, index: usize) -> Option<Triangle3d> {
        self.faces
            .get(index)
            .map(|indices| Self::build_face_triangle(&self.vertices, *indices))
    }

    pub fn face_triangles(&self) -> Vec<Triangle3d> {
        self.faces
            .iter()
            .map(|indices| Self::build_face_triangle(&self.vertices, *indices))
            .collect()
    }
}

fn take_three_u16(slice: &[u16]) -> Option<[usize; 3]> {
    let (output, _) = slice.split_first_chunk::<3>()?;
    Some(output.map(|v| v.into()))
}

fn take_three_u32(slice: &[u32]) -> Option<[usize; 3]> {
    let (output, _) = slice.split_first_chunk::<3>()?;
    // This is probably evil and should be regarded with skepticism
    Some(output.map(|v| v as usize))
}
