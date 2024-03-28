use crate::{mesh::Indices, Mesh};
use bevy_math::Vec3;
use bevy_math::{IndexedFaceMesh, TriangleMesh};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeshConversionError {
    #[error("Source mesh lacks position data")]
    MissingPositions,

    #[error("Source mesh position data is not Float32x3")]
    PositionsFormat,

    #[error("Source mesh lacks face index data")]
    MissingIndices,

    #[error("Index count {count} is not a multiple of 3")]
    IndexCount { count: usize },
}

impl TryFrom<&Mesh> for IndexedFaceMesh {
    type Error = MeshConversionError;
    /// Build an [`IndexedFaceMesh`] from a [`Mesh`].
    ///
    /// This process is both extremely lossy and fallible.
    fn try_from(mesh: &Mesh) -> Result<Self, MeshConversionError> {
        let Some(position_data) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            return Err(MeshConversionError::MissingPositions);
        };
        let Some(positions) = position_data.as_float3() else {
            return Err(MeshConversionError::PositionsFormat);
        };
        let vertices: Vec<Vec3> = positions.iter().map(|pos| (*pos).into()).collect();

        let Some(indices) = mesh.indices() else {
            return Err(MeshConversionError::MissingIndices);
        };

        // If the indices doesn't have a length divisible by 3, then this
        // fails with an error; this is `None` bubbling up from `take_three_u16`
        // or `take_three_u32`.
        let Some(faces): Option<Vec<[usize; 3]>> = (match indices {
            Indices::U16(vec) => vec.as_slice().chunks(3).map(take_three_u16).collect(),
            Indices::U32(vec) => vec.as_slice().chunks(3).map(take_three_u32).collect(),
        }) else {
            return Err(MeshConversionError::IndexCount {
                count: indices.len(),
            });
        };

        Ok(Self::new(vertices, faces))
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

impl TryFrom<&Mesh> for TriangleMesh {
    type Error = MeshConversionError;

    fn try_from(mesh: &Mesh) -> Result<Self, Self::Error> {
        Ok(Self::from(&IndexedFaceMesh::try_from(mesh)?))
    }
}
