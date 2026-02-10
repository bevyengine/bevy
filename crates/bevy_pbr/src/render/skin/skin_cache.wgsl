// Performs skinning and morph target evaluation in a compute shader.
//
// This is only used for meshes that have the `CacheSkin` component, not all
// meshes. See the documentation of that component for details.
//
// We dispatch one instance of this shader per vertex. Each instance binary
// searches the list of skin tasks to find the mesh instance it's skinning, and
// from there looks up the modelview and joint matrices in order to perform the
// skinning and/or morph target evaluation.

#import bevy_pbr::{
    mesh_functions::{mesh_position_local_to_world, mesh_tangent_local_to_world},
    mesh_preprocess_types::MeshInput,
    mesh_types::{CachedSkinnedVertex, MorphAttributes, MorphDescriptor},
    morph::{layer_count, morph_position, weight_at},
    skinning::{inverse_transpose_3x3m, skin_model, skin_normals}
};
#import bevy_render::maths::affine3_to_square

// Specifies the mesh instance that is to be skinned/morphed.
struct SkinTask {
    // The index of the mesh instance in the mesh input uniform buffer.
    mesh_instance_index: u32,
};

// A single unskinned vertex.
//
// The fields of this structure correspond to the standard mesh attributes.
struct UnskinnedVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    tangent: vec4<f32>,
    joint_weights: vec4<f32>,
    joint_indices: vec4<u32>,
};

// All the mesh instances that are to be skinned/morphed.
//
// Note: We use `arrayLength` on this buffer, so make sure its length is exactly
// the length of the underlying array and not the length of the allocation.
@group(0) @binding(0) var<storage> skin_tasks: array<SkinTask>;

// The output buffer that stores all the skinned/morphed vertices.
//
// Note: We use `arrayLength` on this buffer, so make sure its length is exactly
// the length of the underlying array and not the length of the allocation.
@group(0) @binding(1) var<storage, read_write> cached_skinned_vertices: array<CachedSkinnedVertex>;

// The input vertex buffer.
//
// Because it's impractical in Naga/`naga_oil` to have one structure that
// corresponds to every single vertex layout, we simply store this as a flat
// list of words and fetch the fields generically.
@group(0) @binding(2) var<storage> unskinned_vertices: array<u32>;

// The array of mesh input uniforms.
//
// This is the same buffer that the GPU preprocessing shader uses.
@group(0) @binding(3) var<storage> meshes: array<MeshInput>;

// The global array of skinned joint matrices.
@group(0) @binding(4) var<storage> joint_matrices: array<mat4x4<f32>>;

// The global array of morph weights.
//
// If there are no morph targets, this will be a dummy buffer.
@group(0) @binding(5) var<storage> morph_weights: array<f32>;

// The global array of morph displacements.
//
// If there are no morph targets, this will be a dummy buffer.
@group(0) @binding(6) var<storage> morph_targets: array<MorphAttributes>;

// The global array of morph descriptors, which contain metadata about each
// morph target.
//
// If there are no morph targets, this will be a dummy buffer.
@group(0) @binding(7) var<storage> morph_descriptors: array<MorphDescriptor>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let vertex_id = global_id.x;
    if (vertex_id >= arrayLength(&cached_skinned_vertices)) {
        return;
    }

    // Find the skin task, and unpack it.
    let skin_task_index = locate_skin_task(vertex_id);
    let mesh_instance_index = skin_tasks[skin_task_index].mesh_instance_index;

    // Load the vertex we're working on, and unpack it.
    let unskinned_vertex = unpack_vertex(mesh_instance_index, vertex_id);
    var unskinned_position = unskinned_vertex.position;
#ifdef VERTEX_NORMALS
    var unskinned_normal = unskinned_vertex.normal;
#endif  // VERTEX_NORMALS
#ifdef VERTEX_TANGENTS
    var unskinned_tangent = unskinned_vertex.tangent;
#endif  // VERTEX_TANGENTS

    // Process morph targets.
    let vertex_index = vertex_id - meshes[mesh_instance_index].cached_skin_offset;
    let morph_descriptor_index = meshes[mesh_instance_index].morph_descriptor_index;
    if (morph_descriptor_index != 0xffffffffu) {
        let weights_offset = morph_descriptors[morph_descriptor_index].current_weights_offset;
        let weight_count = morph_descriptors[morph_descriptor_index].weight_count;
        let targets_offset = morph_descriptors[morph_descriptor_index].targets_offset;
        let vertex_count = morph_descriptors[morph_descriptor_index].vertex_count;

        for (var weight_index: u32 = 0u; weight_index < weight_count; weight_index += 1u) {
            let weight = morph_weights[weights_offset + weight_index];
            if weight == 0.0 {
                continue;
            }

            let morph_target_index = targets_offset + weight_index * vertex_count + vertex_index;
            unskinned_position += weight * morph_targets[morph_target_index].position;
#ifdef VERTEX_NORMALS
            unskinned_normal += weight * morph_targets[morph_target_index].normal;
#endif  // VERTEX_NORMALS
#ifdef VERTEX_TANGENTS
            unskinned_tangent += vec4(weight * morph_targets[morph_target_index].tangent, 0.0);
#endif  // VERTEX_TANGENTS
        }
    }

    // Compute the resolved skin matrix, if applicable.
    var skin_index = meshes[mesh_instance_index].current_skin_index;
    var world_from_local: mat4x4<f32>;
    if (skin_index != 0xffffffffu) {
        world_from_local = unskinned_vertex.joint_weights.x *
                joint_matrices[skin_index + unskinned_vertex.joint_indices.x]
            + unskinned_vertex.joint_weights.y *
                joint_matrices[skin_index + unskinned_vertex.joint_indices.y]
            + unskinned_vertex.joint_weights.z *
                joint_matrices[skin_index + unskinned_vertex.joint_indices.z]
            + unskinned_vertex.joint_weights.w *
                joint_matrices[skin_index + unskinned_vertex.joint_indices.w];
    } else {
        world_from_local = affine3_to_square(meshes[mesh_instance_index].world_from_local);
    }

    // Skin the position of the vertex.
    cached_skinned_vertices[vertex_id].position = mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(unskinned_position, 1.0)
    ).xyz;

    // Skin the normal of the vertex, if applicable.
#ifdef VERTEX_NORMALS
    cached_skinned_vertices[vertex_id].normal = normalize(
        inverse_transpose_3x3m(
            mat3x3<f32>(
                world_from_local[0].xyz,
                world_from_local[1].xyz,
                world_from_local[2].xyz
            )
        ) * unskinned_normal
    );
#endif  // VERTEX_NORMALS

    // Skin the mikktspace tangent of the vertex, if applicable.
#ifdef VERTEX_TANGENTS
    cached_skinned_vertices[vertex_id].tangent = mesh_tangent_local_to_world(
        world_from_local,
        unskinned_tangent,
        mesh_instance_index
    );
#endif  // VERTEX_TANGENTS
}

// Binary searches the `skin_tasks` array to locate the skin task we're working on.
fn locate_skin_task(vertex_id: u32) -> u32 {
    var skin_task_lo = 0u;
    var skin_task_hi = arrayLength(&skin_tasks);
    var skin_task_mid = 0u;

    while (skin_task_lo < skin_task_hi) {
        skin_task_mid = skin_task_lo + (skin_task_hi - skin_task_lo) / 2;
        let this_mesh_instance_index = skin_tasks[skin_task_mid].mesh_instance_index;

        let skin_offset_start = meshes[this_mesh_instance_index].cached_skin_offset;
        if (vertex_id < skin_offset_start) {
            skin_task_hi = skin_task_mid;
            continue;
        }

        // Do this check because we don't want to read past the end of the buffer.
        if (skin_task_mid == arrayLength(&skin_tasks)) {
            break;
        }

        let next_mesh_instance_index = skin_tasks[skin_task_mid + 1u].mesh_instance_index;
        let skin_offset_end = meshes[next_mesh_instance_index].cached_skin_offset;
        if (vertex_id >= skin_offset_end) {
            skin_task_lo = skin_task_mid + 1u;
            continue;
        }

        break;
    }

    return skin_task_mid;
}

// Fetches an unskinned vertex from the vertex slab.
fn unpack_vertex(mesh_instance_index: u32, vertex_id: u32) -> UnskinnedVertex {
    let vertex_index = vertex_id - meshes[mesh_instance_index].cached_skin_offset;
    let vertex_offset = #{VERTEX_STRIDE} *
        (meshes[mesh_instance_index].first_vertex_index + vertex_index);

    let position_offset = vertex_offset + #{VERTEX_POSITION_OFFSET};
    let position = vec3<f32>(
        bitcast<f32>(unskinned_vertices[position_offset + 0u]),
        bitcast<f32>(unskinned_vertices[position_offset + 1u]),
        bitcast<f32>(unskinned_vertices[position_offset + 2u])
    );

#ifdef VERTEX_NORMALS
    let normal_offset = vertex_offset + #{VERTEX_NORMAL_OFFSET};
    let normal = vec3<f32>(
        bitcast<f32>(unskinned_vertices[normal_offset + 0u]),
        bitcast<f32>(unskinned_vertices[normal_offset + 1u]),
        bitcast<f32>(unskinned_vertices[normal_offset + 2u])
    );
#else   // VERTEX_NORMALS
    let normal = vec3<f32>(0.0);
#endif  // VERTEX_NORMALS

#ifdef VERTEX_TANGENTS
    let tangent_offset = vertex_offset + #{VERTEX_TANGENT_OFFSET};
    let tangent = vec4<f32>(
        bitcast<f32>(unskinned_vertices[tangent_offset + 0u]),
        bitcast<f32>(unskinned_vertices[tangent_offset + 1u]),
        bitcast<f32>(unskinned_vertices[tangent_offset + 2u]),
        bitcast<f32>(unskinned_vertices[tangent_offset + 3u])
    );
#else   // VERTEX_TANGENTS
    let tangent = vec4<f32>(0.0);
#endif  // VERTEX_TANGENTS

#ifdef VERTEX_JOINT_WEIGHTS
    let joint_weight_offset = vertex_offset + #{VERTEX_JOINT_WEIGHT_OFFSET};
    let joint_weights = vec4<f32>(
        bitcast<f32>(unskinned_vertices[joint_weight_offset + 0u]),
        bitcast<f32>(unskinned_vertices[joint_weight_offset + 1u]),
        bitcast<f32>(unskinned_vertices[joint_weight_offset + 2u]),
        bitcast<f32>(unskinned_vertices[joint_weight_offset + 3u])
    );
#else   // VERTEX_JOINT_WEIGHTS
    let joint_weights = vec4<f32>(0.0);
#endif  // VERTEX_JOINT_WEIGHTS

#ifdef VERTEX_JOINT_INDICES
    let joint_index_offset = vertex_offset + #{VERTEX_JOINT_INDEX_OFFSET};
    let joint_indices = vec4<u32>(
        (unskinned_vertices[joint_index_offset + 0u]) & 0x0000ffffu,
        (unskinned_vertices[joint_index_offset + 0u] >> 16u) & 0x0000ffffu,
        (unskinned_vertices[joint_index_offset + 1u]) & 0x0000ffffu,
        (unskinned_vertices[joint_index_offset + 1u] >> 16u) & 0x0000ffffu
    );
#else   // VERTEX_JOINT_INDICES
    let joint_indices = vec4<u32>(0u);
#endif  // VERTEX_JOINT_INDICES

    return UnskinnedVertex(position, normal, tangent, joint_weights, joint_indices);
}
