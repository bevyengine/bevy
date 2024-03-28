#![allow(unsafe_code)]
#![allow(clippy::undocumented_unsafe_blocks)]
// TODO: Remove these once unsafe code is removed from this module

use super::asset::{
    Meshlet, MeshletBoundingSphere, MeshletBoundingSpheres, MeshletLodErrors, MeshletMesh,
};
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_utils::{HashMap, HashSet};
use itertools::Itertools;
use meshopt::{
    build_meshlets, compute_cluster_bounds, compute_meshlet_bounds,
    ffi::{meshopt_Bounds, meshopt_simplifyScale},
    simplify, Meshlets, SimplifyOptions, VertexDataAdapter,
};
use metis::Graph;
use std::{borrow::Cow, iter, ops::Range};

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
        let indices = validate_input_mesh(mesh)?;

        // Split the mesh into an initial list of meshlets (LOD 0)
        let vertex_buffer = mesh.get_vertex_buffer_data();
        let vertex_stride = mesh.get_vertex_size() as usize;
        let vertices = VertexDataAdapter::new(&vertex_buffer, vertex_stride, 0).unwrap();
        let mut meshlets = build_meshlets(&indices, &vertices, 64, 64, 0.0);
        let mut bounding_spheres = meshlets
            .iter()
            .map(|meshlet| compute_meshlet_bounds(meshlet, &vertices))
            .map(convert_meshlet_bounds)
            .map(|bounding_sphere| MeshletBoundingSpheres {
                self_culling: bounding_sphere,
                self_lod: bounding_sphere,
                parent_lod: bounding_sphere,
            })
            .collect::<Vec<_>>();
        let mut lod_errors = iter::repeat(MeshletLodErrors {
            self_: 0.0,
            parent: f32::MAX,
        })
        .take(meshlets.len())
        .collect::<Vec<_>>();
        let worst_case_meshlet_triangles = meshlets
            .meshlets
            .iter()
            .map(|m| m.triangle_count as u64)
            .sum();

        // Build further LODs
        let mut simplification_queue = 0..meshlets.len();
        let mut lod_level = 0;
        while simplification_queue.len() > 1 && lod_level < 10 {
            // For each meshlet build a set of triangle edges
            let triangle_edges_per_meshlet =
                collect_triangle_edges_per_meshlet(simplification_queue.clone(), &meshlets);

            // For each meshlet build a list of connected meshlets (meshlets that share a triangle edge)
            let connected_meshlets_per_meshlet =
                find_connected_meshlets(simplification_queue.clone(), &triangle_edges_per_meshlet);

            // Group meshlets into roughly groups of 4, grouping meshlets with a high number of shared edges
            // http://glaros.dtc.umn.edu/gkhome/fetch/sw/metis/manual.pdf
            let groups = group_meshlets(
                simplification_queue.clone(),
                &connected_meshlets_per_meshlet,
            );

            let next_lod_start = meshlets.len();

            for group_meshlets in groups.values().filter(|group| group.len() > 1) {
                // Simplify the group to ~50% triangle count
                let (simplified_group_indices, mut group_error) =
                    simplify_meshlet_groups(group_meshlets, &meshlets, &vertices);

                // Build a new bounding sphere for the simplified group as a whole for LOD calculations
                let group_bounding_sphere = convert_meshlet_bounds(compute_cluster_bounds(
                    &simplified_group_indices,
                    &vertices,
                ));

                // Adjust error based on the LOD bounding sphere diameter
                group_error /= 2.0 * group_bounding_sphere.radius;

                // Enforce that parent_error >= child_error (we're currently building the parent from its children)
                let group_error = group_meshlets.iter().fold(group_error, |acc, meshlet_id| {
                    acc.max(lod_errors[*meshlet_id].self_)
                });

                // For each meshlet in the group, set their parent error and parent LOD bounding sphere to that of the simplified group
                for meshlet_id in group_meshlets {
                    lod_errors[*meshlet_id].parent = group_error;
                    bounding_spheres[*meshlet_id].parent_lod = group_bounding_sphere;
                }

                // Build new meshlets using the simplified group
                let new_meshlets_count = split_simplified_groups_into_new_meshlets(
                    simplified_group_indices,
                    &vertices,
                    &mut meshlets,
                );

                // Calculate the culling bounding sphere for the new meshlets and set the LOD bounding spheres
                let new_meshlet_ids = (meshlets.len() - new_meshlets_count)..meshlets.len();
                bounding_spheres.extend(
                    new_meshlet_ids
                        .map(|meshlet_id| {
                            compute_meshlet_bounds(meshlets.get(meshlet_id), &vertices)
                        })
                        .map(convert_meshlet_bounds)
                        .map(|bounding_sphere| MeshletBoundingSpheres {
                            self_culling: bounding_sphere,
                            self_lod: group_bounding_sphere,
                            parent_lod: group_bounding_sphere,
                        }),
                );

                // The error for each new meshlet is the error from simplifying the group
                lod_errors.extend(
                    iter::repeat(MeshletLodErrors {
                        self_: group_error,
                        parent: f32::MAX,
                    })
                    .take(new_meshlets_count),
                );
            }

            simplification_queue = next_lod_start..meshlets.len();
            lod_level += 1;
        }

        // Convert meshopt_Meshlet data to a custom format
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
            worst_case_meshlet_triangles,
            vertex_data: vertex_buffer.into(),
            vertex_ids: meshlets.vertices.into(),
            indices: meshlets.triangles.into(),
            meshlets: bevy_meshlets,
            bounding_spheres: bounding_spheres.into(),
            lod_errors: lod_errors.into(),
        })
    }
}

fn validate_input_mesh(mesh: &Mesh) -> Result<Cow<'_, [u32]>, MeshToMeshletMeshConversionError> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return Err(MeshToMeshletMeshConversionError::WrongMeshPrimitiveTopology);
    }

    if mesh.attributes().map(|(id, _)| id).ne([
        Mesh::ATTRIBUTE_POSITION.id,
        Mesh::ATTRIBUTE_NORMAL.id,
        Mesh::ATTRIBUTE_UV_0.id,
        Mesh::ATTRIBUTE_TANGENT.id,
    ]) {
        return Err(MeshToMeshletMeshConversionError::WrongMeshVertexAttributes);
    }

    match mesh.indices() {
        Some(Indices::U32(indices)) => Ok(Cow::Borrowed(indices.as_slice())),
        Some(Indices::U16(indices)) => Ok(indices.iter().map(|i| *i as u32).collect()),
        _ => Err(MeshToMeshletMeshConversionError::MeshMissingIndices),
    }
}

fn collect_triangle_edges_per_meshlet(
    simplification_queue: Range<usize>,
    meshlets: &Meshlets,
) -> HashMap<usize, HashSet<(u32, u32)>> {
    let mut triangle_edges_per_meshlet = HashMap::new();
    for meshlet_id in simplification_queue {
        let meshlet = meshlets.get(meshlet_id);
        let meshlet_triangle_edges = triangle_edges_per_meshlet
            .entry(meshlet_id)
            .or_insert(HashSet::new());
        for i in meshlet.triangles.chunks(3) {
            let v0 = meshlet.vertices[i[0] as usize];
            let v1 = meshlet.vertices[i[1] as usize];
            let v2 = meshlet.vertices[i[2] as usize];
            meshlet_triangle_edges.insert((v0.min(v1), v0.max(v1)));
            meshlet_triangle_edges.insert((v0.min(v2), v0.max(v2)));
            meshlet_triangle_edges.insert((v1.min(v2), v1.max(v2)));
        }
    }
    triangle_edges_per_meshlet
}

fn find_connected_meshlets(
    simplification_queue: Range<usize>,
    triangle_edges_per_meshlet: &HashMap<usize, HashSet<(u32, u32)>>,
) -> HashMap<usize, Vec<(usize, usize)>> {
    let mut connected_meshlets_per_meshlet = HashMap::new();
    for meshlet_id in simplification_queue.clone() {
        connected_meshlets_per_meshlet.insert(meshlet_id, Vec::new());
    }

    for (meshlet_id1, meshlet_id2) in simplification_queue.tuple_combinations() {
        let shared_edge_count = triangle_edges_per_meshlet[&meshlet_id1]
            .intersection(&triangle_edges_per_meshlet[&meshlet_id2])
            .count();
        if shared_edge_count != 0 {
            connected_meshlets_per_meshlet
                .get_mut(&meshlet_id1)
                .unwrap()
                .push((meshlet_id2, shared_edge_count));
            connected_meshlets_per_meshlet
                .get_mut(&meshlet_id2)
                .unwrap()
                .push((meshlet_id1, shared_edge_count));
        }
    }
    connected_meshlets_per_meshlet
}

fn group_meshlets(
    simplification_queue: Range<usize>,
    connected_meshlets_per_meshlet: &HashMap<usize, Vec<(usize, usize)>>,
) -> HashMap<i32, Vec<usize>> {
    let mut xadj = Vec::with_capacity(simplification_queue.len() + 1);
    let mut adjncy = Vec::new();
    let mut adjwgt = Vec::new();
    for meshlet_id in simplification_queue.clone() {
        xadj.push(adjncy.len() as i32);
        for (connected_meshlet_id, shared_edge_count) in
            connected_meshlets_per_meshlet[&meshlet_id].iter().copied()
        {
            adjncy.push((connected_meshlet_id - simplification_queue.start) as i32);
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
            .push(i + simplification_queue.start);
    }
    groups
}

fn simplify_meshlet_groups(
    group_meshlets: &Vec<usize>,
    meshlets: &Meshlets,
    vertices: &VertexDataAdapter<'_>,
) -> (Vec<u32>, f32) {
    // Build a new index buffer into the mesh vertex data by combining all meshlet data in the group
    let mut group_indices = Vec::new();
    for meshlet_id in group_meshlets {
        let meshlet = meshlets.get(*meshlet_id);
        for meshlet_index in meshlet.triangles {
            group_indices.push(meshlet.vertices[*meshlet_index as usize]);
        }
    }

    // Simplify the group to ~50% triangle count
    let mut error = 0.0;
    let simplified_group_indices = simplify(
        &group_indices,
        vertices,
        group_indices.len() / 2,
        0.01,
        SimplifyOptions::LockBorder,
        Some(&mut error),
    );

    // Convert error to object-space
    // TODO: Use high level bindings https://github.com/gwihlidal/meshopt-rs/commit/5d243a89067b459b3b33a197ac94037d96963191
    error *= unsafe {
        meshopt_simplifyScale(
            vertices.pos_ptr(),
            vertices.vertex_count,
            vertices.vertex_stride,
        )
    };

    (simplified_group_indices, error)
}

fn split_simplified_groups_into_new_meshlets(
    simplified_group_indices: Vec<u32>,
    vertices: &VertexDataAdapter<'_>,
    meshlets: &mut Meshlets,
) -> usize {
    let simplified_meshlets = build_meshlets(&simplified_group_indices, vertices, 64, 64, 0.0);
    let new_meshlets_count = simplified_meshlets.len();

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

    new_meshlets_count
}

fn convert_meshlet_bounds(bounds: meshopt_Bounds) -> MeshletBoundingSphere {
    MeshletBoundingSphere {
        center: bounds.center.into(),
        radius: bounds.radius,
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
