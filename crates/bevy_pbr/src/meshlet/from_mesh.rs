use super::asset::{Meshlet, MeshletBoundingSphere, MeshletMesh};
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_utils::{thiserror, HashMap};
use itertools::Itertools;
use meshopt::{
    build_meshlets, compute_meshlet_bounds_decoder, simplify, SimplifyOptions, VertexDataAdapter,
};
use metis::Graph;
use std::borrow::Cow;

impl MeshletMesh {
    /// Process a [`Mesh`] to generate a [`MeshletMesh`].
    ///
    /// This process is very slow, and should be done ahead of time, and not at runtime.
    ///
    /// This function requires the `meshlet_processor` cargo feature.
    ///
    /// The input mesh must:
    /// 1. Use [`PrimitiveTopology::TriangleList`]
    /// 2. Use indices
    /// 3. Have the exact following set of vertex attributes: `{POSITION, NORMAL, UV_0, TANGENT}`
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, MeshToMeshletMeshConversionError> {
        // Validate mesh format
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            return Err(MeshToMeshletMeshConversionError::WrongMeshPrimitiveTopology);
        }
        if mesh.attributes().map(|(id, _)| id).eq([
            Mesh::ATTRIBUTE_POSITION.id,
            Mesh::ATTRIBUTE_NORMAL.id,
            Mesh::ATTRIBUTE_UV_0.id,
            Mesh::ATTRIBUTE_TANGENT.id,
        ]) {
            return Err(MeshToMeshletMeshConversionError::WrongMeshVertexAttributes);
        }
        let indices = match mesh.indices() {
            Some(Indices::U32(indices)) => Cow::Borrowed(indices.as_slice()),
            Some(Indices::U16(indices)) => indices.iter().map(|i| *i as u32).collect(),
            _ => return Err(MeshToMeshletMeshConversionError::MeshMissingIndices),
        };

        // Split the mesh into an initial list of meshlets (LOD 0)
        let vertex_buffer = mesh.get_vertex_buffer_data();
        let vertices = VertexDataAdapter::new(
            &vertex_buffer,
            vertex_buffer_layout.layout().array_stride as usize,
            0,
        )
        .unwrap();
        let mut meshlets = build_meshlets(&indices, &vertices, 64, 64, 0.0);

        // Build further LODs
        let mut simplification_queue = Vec::from_iter(0..meshlets.len());
        while simplification_queue.len() != 1 {
            // For each meshlet build a list of triangle edges
            let mut triangle_edges_per_meshlet = HashMap::new();
            for meshlet_id in simplification_queue.iter().copied() {
                let meshlet = meshlets.get(meshlet_id);
                for (edge_v1, edge_v2) in meshlet.triangles.iter().copied().tuple_windows() {
                    triangle_edges_per_meshlet
                        .entry(meshlet_id)
                        .or_insert(Vec::new())
                        .push((edge_v1.min(edge_v2), edge_v1.max(edge_v2)));
                }
            }

            // For each meshlet build a list of connected meshlets (meshlets that share a triangle edge)
            let mut connected_meshlets_per_meshlet = HashMap::new();
            for (meshlet_id1, meshlet_id2) in simplification_queue.iter().tuple_combinations() {
                let shared_edge_count: u32 = triangle_edges_per_meshlet[meshlet_id1]
                    .iter()
                    .cartesian_product(triangle_edges_per_meshlet[meshlet_id2].iter())
                    .map(|(edge1, edge2)| (*edge1 == *edge2) as u32)
                    .sum();
                if shared_edge_count != 0 {
                    connected_meshlets_per_meshlet
                        .entry(*meshlet_id1)
                        .or_insert(Vec::new())
                        .push((*meshlet_id2, shared_edge_count));
                    connected_meshlets_per_meshlet
                        .entry(*meshlet_id2)
                        .or_insert(Vec::new())
                        .push((*meshlet_id1, shared_edge_count));
                }
            }

            // Group meshlets into roughly groups of 4, grouping meshlets with a high number of shared edges
            // http://glaros.dtc.umn.edu/gkhome/fetch/sw/metis/manual.pdf
            let mut xadj = Vec::with_capacity(simplification_queue.len() + 1);
            let mut adjncy = Vec::new();
            let mut adjwgt = Vec::new();
            for meshlet_id in simplification_queue.iter() {
                xadj.push(adjncy.len() as i32);
                for (connected_meshlet_id, shared_edge_count) in
                    connected_meshlets_per_meshlet[meshlet_id].iter().copied()
                {
                    adjncy.push((connected_meshlet_id - simplification_queue[0]) as i32);
                    adjwgt.push(shared_edge_count as i32);
                }
            }
            xadj.push(adjncy.len() as i32);

            let mut group_per_meshlet = vec![0; simplification_queue.len()];
            Graph::new(1, (simplification_queue.len() / 4) as i32, &xadj, &adjncy)
                .unwrap()
                .set_adjwgt(&adjwgt)
                .part_kway(&mut group_per_meshlet)
                .unwrap();

            let mut groups = HashMap::new();
            for (i, meshlet_group) in group_per_meshlet.into_iter().enumerate() {
                groups
                    .entry(meshlet_group)
                    .or_insert(Vec::new())
                    .push(i + simplification_queue[0]);
            }

            let next_lod_start = meshlets.len();

            for group_meshlets in groups.values() {
                // Simplify each group to ~50% triangle count
                let mut group_indices = Vec::new();
                for meshlet_id in group_meshlets {
                    let meshlet = meshlets.get(*meshlet_id);
                    for meshlet_index in meshlet.triangles {
                        group_indices.push(meshlet.vertices[*meshlet_index as usize]);
                    }
                }

                let mut error = 0.0;
                let simplified_group_indices = simplify(
                    &group_indices,
                    &vertices,
                    group_indices.len() / 6,
                    0.01,
                    SimplifyOptions::LockBorder,
                    Some(&mut error),
                );

                // Build new meshlets using the simplified group
                let simplified_meshlets =
                    build_meshlets(&simplified_group_indices, &vertices, 64, 64, 0.0);
                let vertex_offset = meshlets.vertices.len() as u32;
                let triangle_offset = meshlets.triangles.len() as u32;
                meshlets
                    .vertices
                    .extend_from_slice(&simplified_meshlets.vertices);
                meshlets
                    .triangles
                    .extend_from_slice(&simplified_meshlets.triangles);
                meshlets
                    .meshlets
                    .extend(simplified_meshlets.meshlets.into_iter().map(|mut meshlet| {
                        meshlet.vertex_offset += vertex_offset;
                        meshlet.triangle_offset += triangle_offset;
                        meshlet
                    }));
            }

            simplification_queue.clear();
            simplification_queue.extend(next_lod_start..meshlets.len());
        }

        // Calculate meshlet bounding spheres
        let meshlet_bounding_spheres = meshlets
            .iter()
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

        // Convert to our own meshlet format
        let bevy_meshlets = meshlets
            .meshlets
            .into_iter()
            .map(|m| Meshlet {
                start_vertex_id: m.vertex_offset,
                start_index_id: m.triangle_offset,
                triangle_count: m.triangle_count,
            })
            .collect();

        Ok(Self {
            total_meshlet_indices: 0, // TODO
            vertex_data: vertex_buffer.into(),
            vertex_ids: meshlets.vertices.into(),
            indices: meshlets.triangles.into(),
            meshlets: bevy_meshlets,
            meshlet_bounding_spheres,
        })
    }
}

/// An error produced by [`MeshletMesh::from_mesh`].
#[derive(thiserror::Error, Debug)]
pub enum MeshToMeshletMeshConversionError {
    #[error("Mesh primitive topology was not TriangleList")]
    WrongMeshPrimitiveTopology,
    #[error("Mesh attributes were not {{POSITION, NORMAL, UV_0, TANGENT}}")]
    WrongMeshVertexAttributes,
    #[error("Mesh had no indices")]
    MeshMissingIndices,
}
