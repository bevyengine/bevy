use super::{asset::Meshlet, asset::MeshletBoundingSphere, asset::MeshletMesh};
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_utils::thiserror;
use meshopt::{build_meshlets, compute_meshlet_bounds_decoder, VertexDataAdapter};
use std::{borrow::Cow, iter};

impl MeshletMesh {
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, MeshToMeshletMeshConversionError> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            return Err(MeshToMeshletMeshConversionError::WrongMeshPrimitiveTopology);
        }
        let vertex_buffer_layout = &mesh.get_mesh_vertex_buffer_layout();
        if vertex_buffer_layout.attribute_ids()
            != [
                Mesh::ATTRIBUTE_POSITION.id,
                Mesh::ATTRIBUTE_NORMAL.id,
                Mesh::ATTRIBUTE_UV_0.id,
                Mesh::ATTRIBUTE_TANGENT.id,
            ]
        {
            return Err(MeshToMeshletMeshConversionError::WrongMeshVertexAttributes);
        }
        let indices = match mesh.indices() {
            Some(Indices::U32(indices)) => Cow::Borrowed(indices.as_slice()),
            Some(Indices::U16(indices)) => indices.iter().map(|i| *i as u32).collect(),
            _ => return Err(MeshToMeshletMeshConversionError::MeshMissingIndices),
        };
        let vertex_buffer = mesh.get_vertex_buffer_data();
        let vertices = VertexDataAdapter::new(
            &vertex_buffer,
            vertex_buffer_layout.layout().array_stride as usize,
            0,
        )
        .expect("TODO");

        let mut meshlets = build_meshlets(&indices, &vertices, 126, 64, 0.0);

        // TODO: Parallelize
        let meshlet_bounding_spheres = meshlets
            .iter()
            .map(|meshlet| {
                let bounds = compute_meshlet_bounds_decoder(
                    meshlet,
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                        .unwrap()
                        .as_float3()
                        .unwrap(),
                );
                MeshletBoundingSphere {
                    center: bounds.center.into(),
                    radius: bounds.radius,
                }
            })
            .collect();

        // TODO: Meshoptimizer seems to pad the buffers themselves?
        // Buffer copies need to be in multiples of 4 bytes
        let padding = ((meshlets.triangles.len() + 3) & !0x3) - meshlets.triangles.len();
        meshlets.triangles.extend(iter::repeat(0).take(padding));

        Ok(Self {
            vertex_data: vertex_buffer.into(),
            vertex_ids: meshlets.vertices.into(),
            indices: meshlets.triangles.into(),
            meshlets: meshlets
                .meshlets
                .into_iter()
                .map(|m| Meshlet {
                    start_vertex_id: m.vertex_offset,
                    start_index_id: m.triangle_offset,
                    vertex_count: m.vertex_count, // TODO: We don't actually need this, remove to save space
                    triangle_count: m.triangle_count,
                })
                .collect(),
            meshlet_bounding_spheres,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MeshToMeshletMeshConversionError {
    #[error("Mesh primitive topology was not TriangleList")]
    WrongMeshPrimitiveTopology,
    #[error("Mesh attributes were not {{POSITION, NORMAL, UV_0, TANGENT}}")]
    WrongMeshVertexAttributes,
    #[error("Mesh had no indices")]
    MeshMissingIndices,
}
