use crate::{mesh::Indices, Mesh};
use bevy_math::{primitives::Triangle3d, ShapeSample, Vec3};
use rand::Rng;
use rand_distr::{Distribution, WeightedAliasIndex};

/// A wrapper struct that caches data to allow O(1) sampling from the surface
/// of a mesh. Used via [`Distribution::sample`].
pub struct MeshSampler {
    mesh: AbstractFaceMesh,
    face_distribution: WeightedAliasIndex<f32>,
}

pub struct DistributionError;

impl MeshSampler {
    pub fn from_face_mesh(mesh: AbstractFaceMesh) -> Result<Self, DistributionError> {
        let areas = mesh.faces.iter().map(|t| t.area()).collect();
        let Ok(face_distribution) = WeightedAliasIndex::new(areas) else {
            return Err(DistributionError);
        };

        Ok(Self {
            mesh,
            face_distribution,
        })
    }
}

impl Distribution<Vec3> for MeshSampler {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec3 {
        let face_index = self.face_distribution.sample(rng);
        self.mesh.faces[face_index].sample_interior(rng)
    }
}

pub struct AbstractFaceMesh {
    faces: Vec<Triangle3d>,
}

pub struct InvalidMeshError;

impl AbstractFaceMesh {
    /// Build an [`AbstractFaceMesh`] from a [`Mesh`].
    ///
    /// This process is both extremely lossy and fallible.
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, InvalidMeshError> {
        let Some(position_data) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            return Err(InvalidMeshError);
        };
        let Some(positions) = position_data.as_float3() else {
            return Err(InvalidMeshError);
        };
        let vertices: Vec<Vec3> = positions.iter().map(|pos| (*pos).into()).collect();

        let Some(indices) = mesh.indices() else {
            return Err(InvalidMeshError);
        };

        // If the indices doesn't have a length divisible by 3, then this
        // fails with an error; this is `None` bubbling up from `take_three_u16`
        // or `take_three_u32`.
        let Some(faces): Option<Vec<[u32; 3]>> = (match indices {
            Indices::U16(vec) => vec.as_slice().chunks(3).map(take_three_u16).collect(),
            Indices::U32(vec) => vec.as_slice().chunks(3).map(take_three_u32).collect(),
        }) else {
            return Err(InvalidMeshError);
        };

        // Build triangular faces by looking up the positions of the vertices
        // using the provided indices
        let triangle_faces = faces
            .iter()
            .map(|indices| Self::build_face(&vertices, *indices))
            .collect();

        Ok(Self {
            faces: triangle_faces,
        })
    }

    // Build a face from the indices of its vertices.
    #[inline]
    fn build_face(vertices: &[Vec3], indices: [u32; 3]) -> Triangle3d {
        let vertices = indices.map(|v| vertices[v as usize]);
        Triangle3d { vertices }
    }

    pub fn face(&self, index: usize) -> Option<Triangle3d> {
        self.faces.get(index).copied()
    }
}

fn take_three_u16(slice: &[u16]) -> Option<[u32; 3]> {
    let (output, _) = slice.split_first_chunk::<3>()?;
    Some(output.map(|v| v.into()))
}

fn take_three_u32(slice: &[u32]) -> Option<[u32; 3]> {
    let (output, _) = slice.split_first_chunk::<3>()?;
    Some(*output)
}
