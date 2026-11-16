use crate::MeshAccessError;

use super::{
    DecodedNormals, DecodedPositions, DecodedUvs, Indices, Mesh, MeshVertexAttribute, UvChannel,
};
use thiserror::Error;
use wgpu_types::{PrimitiveTopology, VertexFormat};

struct MikktspaceGeometryHelper<'a> {
    indices: Option<&'a Indices>,
    positions: DecodedPositions<'a>,
    normals: DecodedNormals<'a>,
    uvs: DecodedUvs<'a>,
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
        self.positions
            .get(self.index(face, vert))
            .unwrap()
            .to_array()
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals.get(self.index(face, vert)).unwrap().to_array()
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.uvs.get(self.index(face, vert)).unwrap().to_array()
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

    // The error for an attribute that is missing or can't be read as floats.
    let unreadable = |attribute: MeshVertexAttribute, expected| -> GenerateTangentsError {
        match mesh.try_attribute_option(attribute) {
            Ok(Some(_)) => {
                GenerateTangentsError::InvalidVertexAttributeFormat(attribute.name, expected)
            }
            Ok(None) => GenerateTangentsError::MissingVertexAttribute(attribute.name),
            Err(err) => err.into(),
        }
    };
    let positions = mesh
        .decoded_positions()
        .ok_or_else(|| unreadable(Mesh::ATTRIBUTE_POSITION, VertexFormat::Float32x3))?;
    let normals = mesh
        .decoded_normals()
        .ok_or_else(|| unreadable(Mesh::ATTRIBUTE_NORMAL, VertexFormat::Float32x3))?;
    let uvs = mesh
        .decoded_uvs(UvChannel::Uv0)
        .ok_or_else(|| unreadable(Mesh::ATTRIBUTE_UV_0, VertexFormat::Float32x2))?;

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
