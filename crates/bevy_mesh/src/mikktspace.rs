use crate::MeshAccessError;

use super::{Indices, Mesh, VertexAttributeValues};
use thiserror::Error;
use wgpu_types::{PrimitiveTopology, VertexFormat};

struct MikktspaceGeometryHelper<'a> {
    indices: Option<&'a Indices>,
    positions: &'a Vec<[f32; 3]>,
    normals: &'a Vec<[f32; 3]>,
    uvs: &'a Vec<[f32; 2]>,
    tangents: Vec<[f32; 4]>,
}

impl MikktspaceGeometryHelper<'_> {
    fn index(&self, face: usize, vert: usize) -> usize {
        let index_index = face * 3 + vert;

        match self.indices {
            Some(Indices::U16(indices)) => indices[index_index] as usize,
            Some(Indices::U32(indices)) => indices[index_index] as usize,
            None => index_index,
        }
    }
}

impl bevy_mikktspace::Geometry for MikktspaceGeometryHelper<'_> {
    fn num_faces(&self) -> usize {
        self.indices
            .map(Indices::len)
            .unwrap_or_else(|| self.positions.len())
            / 3
    }

    fn num_vertices_of_face(&self, _: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.positions[self.index(face, vert)]
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals[self.index(face, vert)]
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.uvs[self.index(face, vert)]
    }

    fn set_tangent(
        &mut self,
        tangent_space: Option<bevy_mikktspace::TangentSpace>,
        face: usize,
        vert: usize,
    ) {
        let idx = self.index(face, vert);
        self.tangents[idx] = tangent_space.unwrap_or_default().tangent_encoded();
    }
}

#[derive(Error, Debug)]
/// Failed to generate tangents for the mesh.
pub enum GenerateTangentsError {
    #[error("cannot generate tangents for {0:?}")]
    UnsupportedTopology(PrimitiveTopology),
    #[error("missing indices")]
    MissingIndices,
    #[error("missing vertex attributes '{0}'")]
    MissingVertexAttribute(&'static str),
    #[error("the '{0}' vertex attribute should have {1:?} format")]
    InvalidVertexAttributeFormat(&'static str, VertexFormat),
    #[error("mesh not suitable for tangent generation")]
    MikktspaceError(#[from] bevy_mikktspace::GenerateTangentSpaceError),
    #[error("Mesh access error: {0}")]
    MeshAccessError(#[from] MeshAccessError),
}

pub(crate) fn generate_tangents_for_mesh(
    mesh: &Mesh,
) -> Result<Vec<[f32; 4]>, GenerateTangentsError> {
    match mesh.primitive_topology() {
        PrimitiveTopology::TriangleList => {}
        other => return Err(GenerateTangentsError::UnsupportedTopology(other)),
    };

    let positions = mesh.try_attribute_option(Mesh::ATTRIBUTE_POSITION)?.ok_or(
        GenerateTangentsError::MissingVertexAttribute(Mesh::ATTRIBUTE_POSITION.name),
    )?;
    let VertexAttributeValues::Float32x3(positions) = positions else {
        return Err(GenerateTangentsError::InvalidVertexAttributeFormat(
            Mesh::ATTRIBUTE_POSITION.name,
            VertexFormat::Float32x3,
        ));
    };
    let normals = mesh.try_attribute_option(Mesh::ATTRIBUTE_NORMAL)?.ok_or(
        GenerateTangentsError::MissingVertexAttribute(Mesh::ATTRIBUTE_NORMAL.name),
    )?;
    let VertexAttributeValues::Float32x3(normals) = normals else {
        return Err(GenerateTangentsError::InvalidVertexAttributeFormat(
            Mesh::ATTRIBUTE_NORMAL.name,
            VertexFormat::Float32x3,
        ));
    };
    let uvs = mesh.try_attribute_option(Mesh::ATTRIBUTE_UV_0)?.ok_or(
        GenerateTangentsError::MissingVertexAttribute(Mesh::ATTRIBUTE_UV_0.name),
    )?;
    let VertexAttributeValues::Float32x2(uvs) = uvs else {
        return Err(GenerateTangentsError::InvalidVertexAttributeFormat(
            Mesh::ATTRIBUTE_UV_0.name,
            VertexFormat::Float32x2,
        ));
    };

    let len = positions.len();
    let tangents = vec![[0., 0., 0., 0.]; len];
    let mut mikktspace_mesh = MikktspaceGeometryHelper {
        indices: mesh.try_indices_option()?,
        positions,
        normals,
        uvs,
        tangents,
    };
    bevy_mikktspace::generate_tangents(&mut mikktspace_mesh)?;

    // mikktspace seems to assume left-handedness so we can flip the sign to correct for this
    for tangent in &mut mikktspace_mesh.tangents {
        tangent[3] = -tangent[3];
    }

    Ok(mikktspace_mesh.tangents)
}
