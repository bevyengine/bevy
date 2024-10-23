use super::asset::{
    Meshlet, MeshletBoundingSphere, MeshletBoundingSpheres, MeshletMesh, MeshletSimplificationError,
};
use alloc::borrow::Cow;
use bevy_math::{ops::log2, IVec3, Vec2, Vec3, Vec3Swizzles};
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_utils::HashMap;
use bitvec::{order::Lsb0, vec::BitVec, view::BitView};
use core::iter;
use derive_more::derive::{Display, Error};
use half::f16;
use itertools::Itertools;
use meshopt::{
    build_meshlets,
    ffi::{meshopt_Meshlet, meshopt_simplifyWithAttributes},
    generate_vertex_remap_multi, Meshlets, SimplifyOptions, VertexDataAdapter, VertexStream,
};
use metis::Graph;
use smallvec::SmallVec;

// Aim to have 8 meshlets per group
const TARGET_MESHLETS_PER_GROUP: usize = 8;
// Reject groups that keep over 95% of their original triangles
const SIMPLIFICATION_FAILURE_PERCENTAGE: f32 = 0.95;

/// Default vertex position quantization factor for use with [`MeshletMesh::from_mesh`].
///
/// Snaps vertices to the nearest 1/16th of a centimeter (1/2^4).
pub const DEFAULT_VERTEX_POSITION_QUANTIZATION_FACTOR: u8 = 4;

const CENTIMETERS_PER_METER: f32 = 100.0;

impl MeshletMesh {
    /// Process a [`Mesh`] to generate a [`MeshletMesh`].
    ///
    /// This process is very slow, and should be done ahead of time, and not at runtime.
    ///
    /// # Requirements
    ///
    /// This function requires the `meshlet_processor` cargo feature.
    ///
    /// The input mesh must:
    /// 1. Use [`PrimitiveTopology::TriangleList`]
    /// 2. Use indices
    /// 3. Have the exact following set of vertex attributes: `{POSITION, NORMAL, UV_0}` (tangents can be used in material shaders, but are calculated at runtime and are not stored in the mesh)
    ///
    /// # Vertex precision
    ///
    /// `vertex_position_quantization_factor` is the amount of precision to use when quantizing vertex positions.
    ///
    /// Vertices are snapped to the nearest (1/2^x)th of a centimeter, where x = `vertex_position_quantization_factor`.
    /// E.g. if x = 4, then vertices are snapped to the nearest 1/2^4 = 1/16th of a centimeter.
    ///
    /// Use [`DEFAULT_VERTEX_POSITION_QUANTIZATION_FACTOR`] as a default, adjusting lower to save memory and disk space, and higher to prevent artifacts if needed.
    ///
    /// To ensure that two different meshes do not have cracks between them when placed directly next to each other:
    ///   * Use the same quantization factor when converting each mesh to a meshlet mesh
    ///   * Ensure that their [`bevy_transform::components::Transform::translation`]s are a multiple of 1/2^x centimeters (note that translations are in meters)
    ///   * Ensure that their [`bevy_transform::components::Transform::scale`]s are the same
    ///   * Ensure that their [`bevy_transform::components::Transform::rotation`]s are a multiple of 90 degrees
    pub fn from_mesh(
        mesh: &Mesh,
        vertex_position_quantization_factor: u8,
    ) -> Result<Self, MeshToMeshletMeshConversionError> {
        // Validate mesh format
        let indices = validate_input_mesh(mesh)?;

        // Split the mesh into an initial list of meshlets (LOD 0)
        let vertex_buffer = mesh.create_packed_vertex_buffer_data();
        let vertex_stride = mesh.get_vertex_size() as usize;
        let vertices = VertexDataAdapter::new(&vertex_buffer, vertex_stride, 0).unwrap();
        let mut meshlets = compute_meshlets(&indices, &vertices);
        let mut bounding_spheres = meshlets
            .iter()
            .map(|meshlet| compute_meshlet_bounds(meshlet, &vertices))
            .map(|bounding_sphere| MeshletBoundingSpheres {
                culling_sphere: bounding_sphere,
                lod_group_sphere: bounding_sphere,
                lod_parent_group_sphere: MeshletBoundingSphere {
                    center: Vec3::ZERO,
                    radius: 0.0,
                },
            })
            .collect::<Vec<_>>();
        let mut simplification_errors = iter::repeat(MeshletSimplificationError {
            group_error: f16::ZERO,
            parent_group_error: f16::MAX,
        })
        .take(meshlets.len())
        .collect::<Vec<_>>();

        // Generate a position-only vertex buffer for determining what meshlets are connected for use in grouping
        let (position_only_vertex_count, position_only_vertex_remap) = generate_vertex_remap_multi(
            vertices.vertex_count,
            &[VertexStream::new_with_stride::<Vec3, _>(
                vertex_buffer.as_ptr(),
                vertex_stride,
            )],
            Some(&indices),
        );

        let mut vertex_locks = vec![0; vertices.vertex_count];

        // Build further LODs
        let mut simplification_queue = Vec::from_iter(0..meshlets.len());
        let mut retry_queue = Vec::new();
        while simplification_queue.len() > 1 {
            // For each meshlet build a list of connected meshlets (meshlets that share a vertex)
            let connected_meshlets_per_meshlet = find_connected_meshlets(
                &simplification_queue,
                &meshlets,
                &position_only_vertex_remap,
                position_only_vertex_count,
            );

            // Group meshlets into roughly groups of size TARGET_MESHLETS_PER_GROUP,
            // grouping meshlets with a high number of shared vertices
            let groups = group_meshlets(&connected_meshlets_per_meshlet, &simplification_queue);

            // Lock borders between groups to prevent cracks when simplifying
            lock_group_borders(
                &mut vertex_locks,
                &groups,
                &meshlets,
                &position_only_vertex_remap,
                position_only_vertex_count,
            );

            let next_lod_start = meshlets.len();
            for group_meshlets in groups.into_iter() {
                // If the group only has a single meshlet, we can't simplify it well, so retry later
                if group_meshlets.len() == 1 {
                    retry_queue.push(group_meshlets[0]);
                    continue;
                }

                // Simplify the group to ~50% triangle count
                let Some((simplified_group_indices, mut group_error)) =
                    simplify_meshlet_group(&group_meshlets, &meshlets, &vertices, &vertex_locks)
                else {
                    // Couldn't simplify the group enough, retry its meshlets later
                    retry_queue.extend_from_slice(&group_meshlets);
                    continue;
                };

                // Compute LOD data for the group
                let group_bounding_sphere = compute_lod_group_data(
                    &group_meshlets,
                    &mut group_error,
                    &mut bounding_spheres,
                    &mut simplification_errors,
                );

                // Build new meshlets using the simplified group
                let new_meshlets_count = split_simplified_group_into_new_meshlets(
                    &simplified_group_indices,
                    &vertices,
                    &mut meshlets,
                );

                // Calculate the culling bounding sphere for the new meshlets and set their LOD group data
                let new_meshlet_ids = (meshlets.len() - new_meshlets_count)..meshlets.len();
                bounding_spheres.extend(new_meshlet_ids.clone().map(|meshlet_id| {
                    MeshletBoundingSpheres {
                        culling_sphere: compute_meshlet_bounds(meshlets.get(meshlet_id), &vertices),
                        lod_group_sphere: group_bounding_sphere,
                        lod_parent_group_sphere: MeshletBoundingSphere {
                            center: Vec3::ZERO,
                            radius: 0.0,
                        },
                    }
                }));
                simplification_errors.extend(
                    iter::repeat(MeshletSimplificationError {
                        group_error,
                        parent_group_error: f16::MAX,
                    })
                    .take(new_meshlet_ids.len()),
                );
            }

            // Set simplification queue to the list of newly created (and retrying) meshlets
            simplification_queue.clear();
            simplification_queue.extend(next_lod_start..meshlets.len());
            if !simplification_queue.is_empty() {
                simplification_queue.append(&mut retry_queue);
            }
        }

        // Copy vertex attributes per meshlet and compress
        let mut vertex_positions = BitVec::<u32, Lsb0>::new();
        let mut vertex_normals = Vec::new();
        let mut vertex_uvs = Vec::new();
        let mut bevy_meshlets = Vec::with_capacity(meshlets.len());
        for (i, meshlet) in meshlets.meshlets.iter().enumerate() {
            build_and_compress_per_meshlet_vertex_data(
                meshlet,
                meshlets.get(i).vertices,
                &vertex_buffer,
                vertex_stride,
                &mut vertex_positions,
                &mut vertex_normals,
                &mut vertex_uvs,
                &mut bevy_meshlets,
                vertex_position_quantization_factor,
            );
        }
        vertex_positions.set_uninitialized(false);

        Ok(Self {
            vertex_positions: vertex_positions.into_vec().into(),
            vertex_normals: vertex_normals.into(),
            vertex_uvs: vertex_uvs.into(),
            indices: meshlets.triangles.into(),
            meshlets: bevy_meshlets.into(),
            meshlet_bounding_spheres: bounding_spheres.into(),
            meshlet_simplification_errors: simplification_errors.into(),
        })
    }
}

fn validate_input_mesh(mesh: &Mesh) -> Result<Cow<'_, [u32]>, MeshToMeshletMeshConversionError> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return Err(MeshToMeshletMeshConversionError::WrongMeshPrimitiveTopology);
    }

    if mesh.attributes().map(|(attribute, _)| attribute.id).ne([
        Mesh::ATTRIBUTE_POSITION.id,
        Mesh::ATTRIBUTE_NORMAL.id,
        Mesh::ATTRIBUTE_UV_0.id,
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
    build_meshlets(indices, vertices, 255, 128, 0.0) // Meshoptimizer won't currently let us do 256 vertices
}

fn find_connected_meshlets(
    simplification_queue: &[usize],
    meshlets: &Meshlets,
    position_only_vertex_remap: &[u32],
    position_only_vertex_count: usize,
) -> Vec<Vec<(usize, usize)>> {
    // For each vertex, build a list of all meshlets that use it
    let mut vertices_to_meshlets = vec![Vec::new(); position_only_vertex_count];
    for (meshlet_queue_id, meshlet_id) in simplification_queue.iter().enumerate() {
        let meshlet = meshlets.get(*meshlet_id);
        for index in meshlet.triangles {
            let vertex_id = position_only_vertex_remap[meshlet.vertices[*index as usize] as usize];
            let vertex_to_meshlets = &mut vertices_to_meshlets[vertex_id as usize];
            // Meshlets are added in order, so we can just check the last element to deduplicate,
            // in the case of two triangles sharing the same vertex within a single meshlet
            if vertex_to_meshlets.last() != Some(&meshlet_queue_id) {
                vertex_to_meshlets.push(meshlet_queue_id);
            }
        }
    }

    // For each meshlet pair, count how many vertices they share
    let mut meshlet_pair_to_shared_vertex_count = HashMap::new();
    for vertex_meshlet_ids in vertices_to_meshlets {
        for (meshlet_queue_id1, meshlet_queue_id2) in
            vertex_meshlet_ids.into_iter().tuple_combinations()
        {
            let count = meshlet_pair_to_shared_vertex_count
                .entry((
                    meshlet_queue_id1.min(meshlet_queue_id2),
                    meshlet_queue_id1.max(meshlet_queue_id2),
                ))
                .or_insert(0);
            *count += 1;
        }
    }

    // For each meshlet, gather all other meshlets that share at least one vertex along with their shared vertex count
    let mut connected_meshlets_per_meshlet = vec![Vec::new(); simplification_queue.len()];
    for ((meshlet_queue_id1, meshlet_queue_id2), shared_count) in
        meshlet_pair_to_shared_vertex_count
    {
        // We record both id1->id2 and id2->id1 as adjacency is symmetrical
        connected_meshlets_per_meshlet[meshlet_queue_id1].push((meshlet_queue_id2, shared_count));
        connected_meshlets_per_meshlet[meshlet_queue_id2].push((meshlet_queue_id1, shared_count));
    }

    // The order of meshlets depends on hash traversal order; to produce deterministic results, sort them
    for list in connected_meshlets_per_meshlet.iter_mut() {
        list.sort_unstable();
    }

    connected_meshlets_per_meshlet
}

// METIS manual: https://github.com/KarypisLab/METIS/blob/e0f1b88b8efcb24ffa0ec55eabb78fbe61e58ae7/manual/manual.pdf
fn group_meshlets(
    connected_meshlets_per_meshlet: &[Vec<(usize, usize)>],
    simplification_queue: &[usize],
) -> Vec<SmallVec<[usize; TARGET_MESHLETS_PER_GROUP]>> {
    let mut xadj = Vec::with_capacity(simplification_queue.len() + 1);
    let mut adjncy = Vec::new();
    let mut adjwgt = Vec::new();
    for connected_meshlets in connected_meshlets_per_meshlet {
        xadj.push(adjncy.len() as i32);
        for (connected_meshlet_queue_id, shared_vertex_count) in connected_meshlets {
            adjncy.push(*connected_meshlet_queue_id as i32);
            adjwgt.push(*shared_vertex_count as i32);
            // TODO: Additional weight based on meshlet spatial proximity
        }
    }
    xadj.push(adjncy.len() as i32);

    let mut group_per_meshlet = vec![0; simplification_queue.len()];
    let partition_count = simplification_queue
        .len()
        .div_ceil(TARGET_MESHLETS_PER_GROUP); // TODO: Nanite uses groups of 8-32, probably based on some kind of heuristic
    Graph::new(1, partition_count as i32, &xadj, &adjncy)
        .unwrap()
        .set_option(metis::option::Seed(17))
        .set_adjwgt(&adjwgt)
        .part_kway(&mut group_per_meshlet)
        .unwrap();

    let mut groups = vec![SmallVec::new(); partition_count];
    for (meshlet_queue_id, meshlet_group) in group_per_meshlet.into_iter().enumerate() {
        groups[meshlet_group as usize].push(simplification_queue[meshlet_queue_id]);
    }
    groups
}

fn lock_group_borders(
    vertex_locks: &mut [u8],
    groups: &[SmallVec<[usize; TARGET_MESHLETS_PER_GROUP]>],
    meshlets: &Meshlets,
    position_only_vertex_remap: &[u32],
    position_only_vertex_count: usize,
) {
    let mut position_only_locks = vec![-1; position_only_vertex_count];

    // Iterate over position-only based vertices of all meshlets in all groups
    for (group_id, group_meshlets) in groups.iter().enumerate() {
        for meshlet_id in group_meshlets {
            let meshlet = meshlets.get(*meshlet_id);
            for index in meshlet.triangles {
                let vertex_id =
                    position_only_vertex_remap[meshlet.vertices[*index as usize] as usize] as usize;

                // If the vertex is not yet claimed by any group, or was already claimed by this group
                if position_only_locks[vertex_id] == -1
                    || position_only_locks[vertex_id] == group_id as i32
                {
                    position_only_locks[vertex_id] = group_id as i32; // Then claim the vertex for this group
                } else {
                    position_only_locks[vertex_id] = -2; // Else vertex was already claimed by another group or was already locked, lock it
                }
            }
        }
    }

    // Lock vertices used by more than 1 group
    for i in 0..vertex_locks.len() {
        let vertex_id = position_only_vertex_remap[i] as usize;
        vertex_locks[i] = (position_only_locks[vertex_id] == -2) as u8;
    }
}

#[allow(unsafe_code)]
#[allow(clippy::undocumented_unsafe_blocks)]
fn simplify_meshlet_group(
    group_meshlets: &[usize],
    meshlets: &Meshlets,
    vertices: &VertexDataAdapter<'_>,
    vertex_locks: &[u8],
) -> Option<(Vec<u32>, f16)> {
    // Build a new index buffer into the mesh vertex data by combining all meshlet data in the group
    let mut group_indices = Vec::new();
    for meshlet_id in group_meshlets {
        let meshlet = meshlets.get(*meshlet_id);
        for meshlet_index in meshlet.triangles {
            group_indices.push(meshlet.vertices[*meshlet_index as usize]);
        }
    }

    // Simplify the group to ~50% triangle count
    // TODO: Simplify using vertex attributes
    let mut error = 0.0;
    let simplified_group_indices = unsafe {
        let vertex_data = vertices.reader.get_ref();
        let vertex_data = vertex_data.as_ptr().cast::<u8>();
        let positions = vertex_data.add(vertices.position_offset);
        let mut result: Vec<u32> = vec![0; group_indices.len()];
        let index_count = meshopt_simplifyWithAttributes(
            result.as_mut_ptr().cast(),
            group_indices.as_ptr().cast(),
            group_indices.len(),
            positions.cast::<f32>(),
            vertices.vertex_count,
            vertices.vertex_stride,
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            vertex_locks.as_ptr().cast(),
            group_indices.len() / 2,
            f32::MAX,
            (SimplifyOptions::Sparse | SimplifyOptions::ErrorAbsolute).bits(),
            core::ptr::from_mut(&mut error),
        );
        result.resize(index_count, 0u32);
        result
    };

    // Check if we were able to simplify at least a little
    if simplified_group_indices.len() as f32 / group_indices.len() as f32
        > SIMPLIFICATION_FAILURE_PERCENTAGE
    {
        return None;
    }

    Some((simplified_group_indices, f16::from_f32(error)))
}

fn compute_lod_group_data(
    group_meshlets: &[usize],
    group_error: &mut f16,
    bounding_spheres: &mut [MeshletBoundingSpheres],
    simplification_errors: &mut [MeshletSimplificationError],
) -> MeshletBoundingSphere {
    let mut group_bounding_sphere = MeshletBoundingSphere {
        center: Vec3::ZERO,
        radius: 0.0,
    };

    // Compute the lod group sphere center as a weighted average of the children spheres
    let mut weight = 0.0;
    for meshlet_id in group_meshlets {
        let meshlet_lod_bounding_sphere = bounding_spheres[*meshlet_id].lod_group_sphere;
        group_bounding_sphere.center +=
            meshlet_lod_bounding_sphere.center * meshlet_lod_bounding_sphere.radius;
        weight += meshlet_lod_bounding_sphere.radius;
    }
    group_bounding_sphere.center /= weight;

    // Force parent group sphere to contain all child group spheres (we're currently building the parent from its children)
    // TODO: This does not produce the absolute minimal bounding sphere. Doing so is non-trivial.
    //       "Smallest enclosing balls of balls" http://www.inf.ethz.ch/personal/emo/DoctThesisFiles/fischer05.pdf
    for meshlet_id in group_meshlets {
        let meshlet_lod_bounding_sphere = bounding_spheres[*meshlet_id].lod_group_sphere;
        let d = meshlet_lod_bounding_sphere
            .center
            .distance(group_bounding_sphere.center);
        group_bounding_sphere.radius = group_bounding_sphere
            .radius
            .max(meshlet_lod_bounding_sphere.radius + d);
    }

    // Force parent error to be >= child error (we're currently building the parent from its children)
    for meshlet_id in group_meshlets {
        *group_error = group_error.max(simplification_errors[*meshlet_id].group_error);
    }

    // Set the children's lod parent group data to the new lod group we just made
    for meshlet_id in group_meshlets {
        bounding_spheres[*meshlet_id].lod_parent_group_sphere = group_bounding_sphere;
        simplification_errors[*meshlet_id].parent_group_error = *group_error;
    }

    group_bounding_sphere
}

fn split_simplified_group_into_new_meshlets(
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

#[allow(clippy::too_many_arguments)]
fn build_and_compress_per_meshlet_vertex_data(
    meshlet: &meshopt_Meshlet,
    meshlet_vertex_ids: &[u32],
    vertex_buffer: &[u8],
    vertex_stride: usize,
    vertex_positions: &mut BitVec<u32, Lsb0>,
    vertex_normals: &mut Vec<u32>,
    vertex_uvs: &mut Vec<Vec2>,
    meshlets: &mut Vec<Meshlet>,
    vertex_position_quantization_factor: u8,
) {
    let start_vertex_position_bit = vertex_positions.len() as u32;
    let start_vertex_attribute_id = vertex_normals.len() as u32;

    let quantization_factor =
        (1 << vertex_position_quantization_factor) as f32 * CENTIMETERS_PER_METER;

    let mut min_quantized_position_channels = IVec3::MAX;
    let mut max_quantized_position_channels = IVec3::MIN;

    // Lossy vertex compression
    let mut quantized_positions = [IVec3::ZERO; 255];
    for (i, vertex_id) in meshlet_vertex_ids.iter().enumerate() {
        // Load source vertex attributes
        let vertex_id_byte = *vertex_id as usize * vertex_stride;
        let vertex_data = &vertex_buffer[vertex_id_byte..(vertex_id_byte + vertex_stride)];
        let position = Vec3::from_slice(bytemuck::cast_slice(&vertex_data[0..12]));
        let normal = Vec3::from_slice(bytemuck::cast_slice(&vertex_data[12..24]));
        let uv = Vec2::from_slice(bytemuck::cast_slice(&vertex_data[24..32]));

        // Copy uncompressed UV
        vertex_uvs.push(uv);

        // Compress normal
        vertex_normals.push(pack2x16snorm(octahedral_encode(normal)));

        // Quantize position to a fixed-point IVec3
        let quantized_position = (position * quantization_factor + 0.5).as_ivec3();
        quantized_positions[i] = quantized_position;

        // Compute per X/Y/Z-channel quantized position min/max for this meshlet
        min_quantized_position_channels = min_quantized_position_channels.min(quantized_position);
        max_quantized_position_channels = max_quantized_position_channels.max(quantized_position);
    }

    // Calculate bits needed to encode each quantized vertex position channel based on the range of each channel
    let range = max_quantized_position_channels - min_quantized_position_channels + 1;
    let bits_per_vertex_position_channel_x = log2(range.x as f32).ceil() as u8;
    let bits_per_vertex_position_channel_y = log2(range.y as f32).ceil() as u8;
    let bits_per_vertex_position_channel_z = log2(range.z as f32).ceil() as u8;

    // Lossless encoding of vertex positions in the minimum number of bits per channel
    for quantized_position in quantized_positions.iter().take(meshlet_vertex_ids.len()) {
        // Remap [range_min, range_max] IVec3 to [0, range_max - range_min] UVec3
        let position = (quantized_position - min_quantized_position_channels).as_uvec3();

        // Store as a packed bitstream
        vertex_positions.extend_from_bitslice(
            &position.x.view_bits::<Lsb0>()[..bits_per_vertex_position_channel_x as usize],
        );
        vertex_positions.extend_from_bitslice(
            &position.y.view_bits::<Lsb0>()[..bits_per_vertex_position_channel_y as usize],
        );
        vertex_positions.extend_from_bitslice(
            &position.z.view_bits::<Lsb0>()[..bits_per_vertex_position_channel_z as usize],
        );
    }

    meshlets.push(Meshlet {
        start_vertex_position_bit,
        start_vertex_attribute_id,
        start_index_id: meshlet.triangle_offset,
        vertex_count: meshlet.vertex_count as u8,
        triangle_count: meshlet.triangle_count as u8,
        padding: 0,
        bits_per_vertex_position_channel_x,
        bits_per_vertex_position_channel_y,
        bits_per_vertex_position_channel_z,
        vertex_position_quantization_factor,
        min_vertex_position_channel_x: min_quantized_position_channels.x as f32,
        min_vertex_position_channel_y: min_quantized_position_channels.y as f32,
        min_vertex_position_channel_z: min_quantized_position_channels.z as f32,
    });
}

fn compute_meshlet_bounds(
    meshlet: meshopt::Meshlet<'_>,
    vertices: &VertexDataAdapter<'_>,
) -> MeshletBoundingSphere {
    let bounds = meshopt::compute_meshlet_bounds(meshlet, vertices);
    MeshletBoundingSphere {
        center: bounds.center.into(),
        radius: bounds.radius,
    }
}

// TODO: Precise encode variant
fn octahedral_encode(v: Vec3) -> Vec2 {
    let n = v / (v.x.abs() + v.y.abs() + v.z.abs());
    let octahedral_wrap = (1.0 - n.yx().abs())
        * Vec2::new(
            if n.x >= 0.0 { 1.0 } else { -1.0 },
            if n.y >= 0.0 { 1.0 } else { -1.0 },
        );
    if n.z >= 0.0 {
        n.xy()
    } else {
        octahedral_wrap
    }
}

// https://www.w3.org/TR/WGSL/#pack2x16snorm-builtin
fn pack2x16snorm(v: Vec2) -> u32 {
    let v = v.clamp(Vec2::NEG_ONE, Vec2::ONE);
    let v = (v * 32767.0 + 0.5).floor().as_i16vec2();
    bytemuck::cast(v)
}

/// An error produced by [`MeshletMesh::from_mesh`].
#[derive(Error, Display, Debug)]
pub enum MeshToMeshletMeshConversionError {
    #[display("Mesh primitive topology is not TriangleList")]
    WrongMeshPrimitiveTopology,
    #[display("Mesh attributes are not {{POSITION, NORMAL, UV_0}}")]
    WrongMeshVertexAttributes,
    #[display("Mesh has no indices")]
    MeshMissingIndices,
}
