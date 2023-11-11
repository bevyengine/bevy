use super::asset::{Meshlet, MeshletBoundingSphere, MeshletMesh};
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_utils::thiserror;
use meshopt::{build_meshlets, compute_meshlet_bounds_decoder, VertexDataAdapter};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
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

        let mut meshopt_meshlets = build_meshlets(&indices, &vertices, 126, 64, 0.0);

        let meshlet_bounding_spheres = meshopt_meshlets
            .meshlets
            .par_iter()
            .map(|meshlet| meshopt::Meshlet {
                vertices: &meshopt_meshlets.vertices[meshlet.vertex_offset as usize
                    ..meshlet.vertex_offset as usize + meshlet.vertex_count as usize],
                triangles: &meshopt_meshlets.triangles[meshlet.triangle_offset as usize
                    ..meshlet.triangle_offset as usize + meshlet.triangle_count as usize * 3],
            })
            .map(|meshlet| {
                compute_meshlet_bounds_decoder(
                    meshlet,
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                        .unwrap()
                        .as_float3()
                        .unwrap(),
                )
            })
            .map(|bounds| MeshletBoundingSphere {
                center: bounds.center.into(),
                radius: bounds.radius,
            })
            .collect();

        // TODO: Meshoptimizer seems to pad the buffers themselves?
        // Buffer copies need to be in multiples of 4 bytes
        let padding =
            ((meshopt_meshlets.triangles.len() + 3) & !0x3) - meshopt_meshlets.triangles.len();
        meshopt_meshlets
            .triangles
            .extend(iter::repeat(0).take(padding));

        let mut total_meshlet_indices = 0;
        let meshlets = meshopt_meshlets
            .meshlets
            .into_iter()
            .map(|m| {
                total_meshlet_indices += m.triangle_count as u64 * 3;
                Meshlet {
                    start_vertex_id: m.vertex_offset,
                    start_index_id: m.triangle_offset,
                    index_count: m.triangle_count * 3,
                }
            })
            .collect();

        Ok(Self {
            total_meshlet_indices,
            vertex_data: vertex_buffer.into(),
            vertex_ids: meshopt_meshlets.vertices.into(),
            indices: meshopt_meshlets.triangles.into(),
            meshlets,
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
