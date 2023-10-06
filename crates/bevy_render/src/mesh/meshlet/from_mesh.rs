use super::{Meshlet, MeshletBoundingCone, MeshletBoundingSphere, MeshletMesh};
use crate::mesh::{Indices, Mesh};
use bevy_asset::anyhow::{bail, Error};
use meshopt::{build_meshlets, compute_meshlet_bounds_decoder, VertexDataAdapter};
use wgpu::PrimitiveTopology;

impl MeshletMesh {
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, Error> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            bail!("Mesh primitive_topology was not TriangleList");
        }
        let vertex_buffer_layout = &mesh.get_mesh_vertex_buffer_layout();
        if vertex_buffer_layout.attribute_ids()
            != &[
                Mesh::ATTRIBUTE_POSITION.id,
                Mesh::ATTRIBUTE_NORMAL.id,
                Mesh::ATTRIBUTE_UV_0.id,
                Mesh::ATTRIBUTE_TANGENT.id,
            ]
        {
            bail!("Mesh attributes were not {{POSITION, NORMAL, UV_0, TANGENT}}");
        }
        let indices = match mesh.indices() {
            Some(Indices::U32(indices)) => indices,
            _ => bail!("Mesh indicies were not Some(Indices::U32)"),
        };
        let vertex_buffer = mesh.get_vertex_buffer_data();
        let vertices = VertexDataAdapter::new(
            &vertex_buffer,
            vertex_buffer_layout.layout().array_stride as usize,
            0,
        )?;

        let meshlets = build_meshlets(indices, &vertices, 126, 64, 0.5);

        let mut meshlet_bounding_spheres = Vec::with_capacity(meshlets.meshlets.len());
        let mut meshlet_bounding_cones = Vec::with_capacity(meshlets.meshlets.len());
        for meshlet in meshlets.iter() {
            let bounds = compute_meshlet_bounds_decoder(
                meshlet,
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    .unwrap()
                    .as_float3()
                    .unwrap(),
            );
            meshlet_bounding_spheres.push(MeshletBoundingSphere {
                center: bounds.center.into(),
                radius: bounds.radius,
            });
            meshlet_bounding_cones.push(MeshletBoundingCone {
                apex: bounds.cone_apex.into(),
                axis: bounds.cone_axis.into(),
            });
        }

        Ok(Self {
            mesh_vertex_data: vertex_buffer.into_boxed_slice(),
            meshlet_vertex_buffer: meshlets.vertices.into_boxed_slice(),
            meshlet_index_buffer: meshlets.triangles.into_boxed_slice(),
            meshlets: meshlets
                .meshlets
                .into_iter()
                .map(|m| Meshlet {
                    meshlet_vertex_buffer_index: m.vertex_offset,
                    meshlet_index_buffer_index: m.triangle_offset,
                    meshlet_vertex_count: m.vertex_count,
                    meshlet_triangle_count: m.triangle_count,
                })
                .collect(),
            meshlet_bounding_spheres: meshlet_bounding_spheres.into_boxed_slice(),
            meshlet_bounding_cones: meshlet_bounding_cones.into_boxed_slice(),
        })
    }
}
