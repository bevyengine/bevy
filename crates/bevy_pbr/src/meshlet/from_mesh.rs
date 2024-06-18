use super::asset::{Meshlet, MeshletBoundingSphere, MeshletBoundingSpheres, MeshletMesh};
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_utils::HashMap;
use itertools::Itertools;
use meshopt::{
    build_meshlets, compute_cluster_bounds, compute_meshlet_bounds,
    ffi::{meshopt_Bounds, meshopt_optimizeMeshlet},
    simplify, simplify_scale, Meshlets, SimplifyOptions, VertexDataAdapter,
};
use metis::Graph;
use smallvec::SmallVec;
use std::{borrow::Cow, ops::Range};

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
        let mut meshlets = compute_meshlets(&indices, &vertices);
        let mut bounding_spheres = meshlets
            .iter()
            .map(|meshlet| compute_meshlet_bounds(meshlet, &vertices))
            .map(convert_meshlet_bounds)
            .map(|bounding_sphere| MeshletBoundingSpheres {
                self_culling: bounding_sphere,
                self_lod: MeshletBoundingSphere {
                    center: bounding_sphere.center,
                    radius: 0.0,
                },
                parent_lod: MeshletBoundingSphere {
                    center: bounding_sphere.center,
                    radius: f32::MAX,
                },
            })
            .collect::<Vec<_>>();
        let worst_case_meshlet_triangles = meshlets
            .meshlets
            .iter()
            .map(|m| m.triangle_count as u64)
            .sum();
        let mesh_scale = simplify_scale(&vertices);

        // Build further LODs
        let mut simplification_queue = 0..meshlets.len();
        let mut lod_level = 1;
        while simplification_queue.len() > 1 {
            // For each meshlet build a list of connected meshlets (meshlets that share a triangle edge)
            let connected_meshlets_per_meshlet =
                find_connected_meshlets(simplification_queue.clone(), &meshlets);

            // Group meshlets into roughly groups of 4, grouping meshlets with a high number of shared edges
            // http://glaros.dtc.umn.edu/gkhome/fetch/sw/metis/manual.pdf
            let groups = group_meshlets(
                simplification_queue.clone(),
                &connected_meshlets_per_meshlet,
            );

            let next_lod_start = meshlets.len();

            for group_meshlets in groups.into_iter().filter(|group| group.len() > 1) {
                // Simplify the group to ~50% triangle count
                let Some((simplified_group_indices, mut group_error)) = simplify_meshlet_groups(
                    &group_meshlets,
                    &meshlets,
                    &vertices,
                    lod_level,
                    mesh_scale,
                ) else {
                    continue;
                };

                // Add the maximum child error to the parent error to make parent error cumulative from LOD 0
                // (we're currently building the parent from its children)
                group_error += group_meshlets.iter().fold(group_error, |acc, meshlet_id| {
                    acc.max(bounding_spheres[*meshlet_id].self_lod.radius)
                });

                // Build a new LOD bounding sphere for the simplified group as a whole
                let mut group_bounding_sphere = convert_meshlet_bounds(compute_cluster_bounds(
                    &simplified_group_indices,
                    &vertices,
                ));
                group_bounding_sphere.radius = group_error;

                // For each meshlet in the group set their parent LOD bounding sphere to that of the simplified group
                for meshlet_id in group_meshlets {
                    bounding_spheres[meshlet_id].parent_lod = group_bounding_sphere;
                }

                // Build new meshlets using the simplified group
                let new_meshlets_count = split_simplified_groups_into_new_meshlets(
                    &simplified_group_indices,
                    &vertices,
                    &mut meshlets,
                );

                // Calculate the culling bounding sphere for the new meshlets and set their LOD bounding spheres
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
                            parent_lod: MeshletBoundingSphere {
                                center: group_bounding_sphere.center,
                                radius: f32::MAX,
                            },
                        }),
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

fn compute_meshlets(indices: &[u32], vertices: &VertexDataAdapter) -> Meshlets {
    let mut meshlets = build_meshlets(indices, vertices, 64, 64, 0.0);

    for meshlet in &mut meshlets.meshlets {
        #[allow(unsafe_code)]
        #[allow(clippy::undocumented_unsafe_blocks)]
        unsafe {
            meshopt_optimizeMeshlet(
                &mut meshlets.vertices[meshlet.vertex_offset as usize],
                &mut meshlets.triangles[meshlet.triangle_offset as usize],
                meshlet.triangle_count as usize,
                meshlet.vertex_count as usize,
            );
        }
    }

    meshlets
}

fn find_connected_meshlets(
    simplification_queue: Range<usize>,
    meshlets: &Meshlets,
) -> Vec<Vec<(usize, usize)>> {
    // For each edge, gather all meshlets that use it
    let mut edges_to_meshlets = HashMap::new();

    for meshlet_id in simplification_queue.clone() {
        let meshlet = meshlets.get(meshlet_id);
        for i in meshlet.triangles.chunks(3) {
            for k in 0..3 {
                let v0 = meshlet.vertices[i[k] as usize];
                let v1 = meshlet.vertices[i[(k + 1) % 3] as usize];
                let edge = (v0.min(v1), v0.max(v1));

                let vec = edges_to_meshlets
                    .entry(edge)
                    .or_insert_with(SmallVec::<[usize; 2]>::new);
                // Meshlets are added in order, so we can just check the last element to deduplicate,
                // in the case of two triangles sharing the same edge within a single meshlet
                if vec.last() != Some(&meshlet_id) {
                    vec.push(meshlet_id);
                }
            }
        }
    }

    // For each meshlet pair, count how many edges they share
    let mut shared_edge_count = HashMap::new();

    for (_, meshlet_ids) in edges_to_meshlets {
        for (meshlet_id1, meshlet_id2) in meshlet_ids.into_iter().tuple_combinations() {
            let count = shared_edge_count
                .entry((meshlet_id1.min(meshlet_id2), meshlet_id1.max(meshlet_id2)))
                .or_insert(0);
            *count += 1;
        }
    }

    // For each meshlet, gather all meshlets that share at least one edge along with shared edge count
    let mut connected_meshlets = vec![Vec::new(); simplification_queue.len()];

    for ((meshlet_id1, meshlet_id2), shared_count) in shared_edge_count {
        // We record id1->id2 and id2->id1 as adjacency is symmetrical
        connected_meshlets[meshlet_id1 - simplification_queue.start]
            .push((meshlet_id2, shared_count));
        connected_meshlets[meshlet_id2 - simplification_queue.start]
            .push((meshlet_id1, shared_count));
    }

    // The order of meshlets depends on hash traversal order; to produce deterministic results, sort them
    for list in connected_meshlets.iter_mut() {
        list.sort_unstable();
    }

    connected_meshlets
}

fn group_meshlets(
    simplification_queue: Range<usize>,
    connected_meshlets_per_meshlet: &[Vec<(usize, usize)>],
) -> Vec<Vec<usize>> {
    let mut xadj = Vec::with_capacity(simplification_queue.len() + 1);
    let mut adjncy = Vec::new();
    let mut adjwgt = Vec::new();
    for meshlet_id in simplification_queue.clone() {
        xadj.push(adjncy.len() as i32);
        for (connected_meshlet_id, shared_edge_count) in
            connected_meshlets_per_meshlet[meshlet_id - simplification_queue.start].iter()
        {
            adjncy.push((connected_meshlet_id - simplification_queue.start) as i32);
            adjwgt.push(*shared_edge_count as i32);
        }
    }
    xadj.push(adjncy.len() as i32);

    let mut group_per_meshlet = vec![0; simplification_queue.len()];
    let partition_count = simplification_queue.len().div_ceil(4);
    Graph::new(1, partition_count as i32, &xadj, &adjncy)
        .unwrap()
        .set_adjwgt(&adjwgt)
        .part_kway(&mut group_per_meshlet)
        .unwrap();

    let mut groups = vec![Vec::new(); partition_count];

    for (i, meshlet_group) in group_per_meshlet.into_iter().enumerate() {
        groups[meshlet_group as usize].push(i + simplification_queue.start);
    }
    groups
}

fn simplify_meshlet_groups(
    group_meshlets: &[usize],
    meshlets: &Meshlets,
    vertices: &VertexDataAdapter<'_>,
    lod_level: u32,
    mesh_scale: f32,
) -> Option<(Vec<u32>, f32)> {
    // Build a new index buffer into the mesh vertex data by combining all meshlet data in the group
    let mut group_indices = Vec::new();
    for meshlet_id in group_meshlets {
        let meshlet = meshlets.get(*meshlet_id);
        for meshlet_index in meshlet.triangles {
            group_indices.push(meshlet.vertices[*meshlet_index as usize]);
        }
    }

    // Allow more deformation for high LOD levels (1% at LOD 1, 10% at LOD 20+)
    let t = (lod_level - 1) as f32 / 19.0;
    let target_error = 0.1 * t + 0.01 * (1.0 - t);

    // Simplify the group to ~50% triangle count
    // TODO: Use simplify_with_locks()
    let mut error = 0.0;
    let simplified_group_indices = simplify(
        &group_indices,
        vertices,
        group_indices.len() / 2,
        target_error,
        SimplifyOptions::LockBorder,
        Some(&mut error),
    );

    // Check if we were able to simplify to at least 65% triangle count
    if simplified_group_indices.len() as f32 / group_indices.len() as f32 > 0.65 {
        return None;
    }

    // Convert error to object-space and convert from diameter to radius
    error *= mesh_scale * 0.5;

    Some((simplified_group_indices, error))
}

fn split_simplified_groups_into_new_meshlets(
    simplified_group_indices: &[u32],
    vertices: &VertexDataAdapter<'_>,
    meshlets: &mut Meshlets,
) -> usize {
    let simplified_meshlets = compute_meshlets(simplified_group_indices, vertices);
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
    #[error("Mesh primitive topology is not TriangleList")]
    WrongMeshPrimitiveTopology,
    #[error("Mesh attributes are not {{POSITION, NORMAL, UV_0, TANGENT}}")]
    WrongMeshVertexAttributes,
    #[error("Mesh has no indices")]
    MeshMissingIndices,
}
