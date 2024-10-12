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
use core::{iter, ops::Range};
use derive_more::derive::{Display, Error};
use half::f16;
use itertools::Itertools;
use meshopt::{
    build_meshlets, ffi::meshopt_Meshlet, simplify, Meshlets, SimplifyOptions, VertexDataAdapter,
};
use metis::Graph;
use smallvec::SmallVec;

// Aim to have 8 meshlets per group
const TARGET_MESHLETS_PER_GROUP: usize = 8;
// Reject groups that keep at least 95% of their original triangles
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
    /// `vertex_position_quantization_factor` is the amount of precision to to use when quantizing vertex positions.
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

        // Build further LODs
        let mut simplification_queue = 0..meshlets.len();
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
                let Some((simplified_group_indices, mut group_error)) =
                    simplify_meshlet_group(&group_meshlets, &meshlets, &vertices)
                else {
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

            simplification_queue = next_lod_start..meshlets.len();
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
) -> Vec<SmallVec<[usize; TARGET_MESHLETS_PER_GROUP]>> {
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
    for (i, meshlet_group) in group_per_meshlet.into_iter().enumerate() {
        groups[meshlet_group as usize].push(i + simplification_queue.start);
    }
    groups
}

fn simplify_meshlet_group(
    group_meshlets: &[usize],
    meshlets: &Meshlets,
    vertices: &VertexDataAdapter<'_>,
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
    let simplified_group_indices = simplify(
        &group_indices,
        vertices,
        group_indices.len() / 2,
        f32::MAX,
        SimplifyOptions::LockBorder | SimplifyOptions::Sparse | SimplifyOptions::ErrorAbsolute, /* TODO: Specify manual vertex locks instead of meshopt's overly-strict locks */
        Some(&mut error),
    );

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
