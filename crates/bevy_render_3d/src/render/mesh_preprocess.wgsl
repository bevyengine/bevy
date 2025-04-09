// GPU mesh transforming and culling.
//
// This is a compute shader that expands each `MeshInputUniform` out to a full
// `MeshUniform` for each view before rendering. (Thus `MeshInputUniform` and
// `MeshUniform` are in a 1:N relationship.) It runs in parallel for all meshes
// for all views. As part of this process, the shader gathers each mesh's
// transform on the previous frame and writes it into the `MeshUniform` so that
// TAA works. It also performs frustum culling and occlusion culling, if
// requested.
//
// If occlusion culling is on, this shader runs twice: once to prepare the
// meshes that were visible last frame, and once to prepare the meshes that
// weren't visible last frame but became visible this frame. The two invocations
// are known as *early mesh preprocessing* and *late mesh preprocessing*
// respectively.

#import bevy_pbr::mesh_preprocess_types::{
    IndirectParametersCpuMetadata, IndirectParametersGpuMetadata, MeshInput
}
#import bevy_pbr::mesh_types::{Mesh, MESH_FLAGS_NO_FRUSTUM_CULLING_BIT}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::occlusion_culling
#import bevy_pbr::prepass_bindings::previous_view_uniforms
#import bevy_pbr::view_transformations::{
    position_world_to_ndc, position_world_to_view, ndc_to_uv, view_z_to_depth_ndc,
    position_world_to_prev_ndc, position_world_to_prev_view, prev_view_z_to_depth_ndc
}
#import bevy_render::maths
#import bevy_render::view::View

// Information about each mesh instance needed to cull it on GPU.
//
// At the moment, this just consists of its axis-aligned bounding box (AABB).
struct MeshCullingData {
    // The 3D center of the AABB in model space, padded with an extra unused
    // float value.
    aabb_center: vec4<f32>,
    // The 3D extents of the AABB in model space, divided by two, padded with
    // an extra unused float value.
    aabb_half_extents: vec4<f32>,
}

// One invocation of this compute shader: i.e. one mesh instance in a view.
struct PreprocessWorkItem {
    // The index of the `MeshInput` in the `current_input` buffer that we read
    // from.
    input_index: u32,
    // In direct mode, the index of the `Mesh` in `output` that we write to. In
    // indirect mode, the index of the `IndirectParameters` in
    // `indirect_parameters` that we write to.
    output_or_indirect_parameters_index: u32,
}

// The parameters for the indirect compute dispatch for the late mesh
// preprocessing phase.
struct LatePreprocessWorkItemIndirectParameters {
    // The number of workgroups we're going to dispatch.
    //
    // This value should always be equal to `ceil(work_item_count / 64)`.
    dispatch_x: atomic<u32>,
    // The number of workgroups in the Y direction; always 1.
    dispatch_y: u32,
    // The number of workgroups in the Z direction; always 1.
    dispatch_z: u32,
    // The precise number of work items.
    work_item_count: atomic<u32>,
    // Padding.
    //
    // This isn't the usual structure padding; it's needed because some hardware
    // requires indirect compute dispatch parameters to be aligned on 64-byte
    // boundaries.
    pad: vec4<u32>,
}

// These have to be in a structure because of Naga limitations on DX12.
struct PushConstants {
    // The offset into the `late_preprocess_work_item_indirect_parameters`
    // buffer.
    late_preprocess_work_item_indirect_offset: u32,
}

// The current frame's `MeshInput`.
@group(0) @binding(3) var<storage> current_input: array<MeshInput>;
// The `MeshInput` values from the previous frame.
@group(0) @binding(4) var<storage> previous_input: array<MeshInput>;
// Indices into the `MeshInput` buffer.
//
// There may be many indices that map to the same `MeshInput`.
@group(0) @binding(5) var<storage> work_items: array<PreprocessWorkItem>;
// The output array of `Mesh`es.
@group(0) @binding(6) var<storage, read_write> output: array<Mesh>;

#ifdef INDIRECT
// The array of indirect parameters for drawcalls.
@group(0) @binding(7) var<storage> indirect_parameters_cpu_metadata:
    array<IndirectParametersCpuMetadata>;

@group(0) @binding(8) var<storage, read_write> indirect_parameters_gpu_metadata:
    array<IndirectParametersGpuMetadata>;
#endif

#ifdef FRUSTUM_CULLING
// Data needed to cull the meshes.
//
// At the moment, this consists only of AABBs.
@group(0) @binding(9) var<storage> mesh_culling_data: array<MeshCullingData>;
#endif  // FRUSTUM_CULLING

#ifdef OCCLUSION_CULLING
@group(0) @binding(10) var depth_pyramid: texture_2d<f32>;

#ifdef EARLY_PHASE
@group(0) @binding(11) var<storage, read_write> late_preprocess_work_items:
    array<PreprocessWorkItem>;
#endif  // EARLY_PHASE

@group(0) @binding(12) var<storage, read_write> late_preprocess_work_item_indirect_parameters:
    array<LatePreprocessWorkItemIndirectParameters>;

var<push_constant> push_constants: PushConstants;
#endif  // OCCLUSION_CULLING

#ifdef FRUSTUM_CULLING
// Returns true if the view frustum intersects an oriented bounding box (OBB).
//
// `aabb_center.w` should be 1.0.
fn view_frustum_intersects_obb(
    world_from_local: mat4x4<f32>,
    aabb_center: vec4<f32>,
    aabb_half_extents: vec3<f32>,
) -> bool {

    for (var i = 0; i < 5; i += 1) {
        // Calculate relative radius of the sphere associated with this plane.
        let plane_normal = view.frustum[i];
        let relative_radius = dot(
            abs(
                vec3(
                    dot(plane_normal.xyz, world_from_local[0].xyz),
                    dot(plane_normal.xyz, world_from_local[1].xyz),
                    dot(plane_normal.xyz, world_from_local[2].xyz),
                )
            ),
            aabb_half_extents
        );

        // Check the frustum plane.
        if (!maths::sphere_intersects_plane_half_space(
                plane_normal, aabb_center, relative_radius)) {
            return false;
        }
    }

    return true;
}
#endif

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    // Figure out our instance index. If this thread doesn't correspond to any
    // index, bail.
    let instance_index = global_invocation_id.x;

#ifdef LATE_PHASE
    if (instance_index >= atomicLoad(&late_preprocess_work_item_indirect_parameters[
            push_constants.late_preprocess_work_item_indirect_offset].work_item_count)) {
        return;
    }
#else   // LATE_PHASE
    if (instance_index >= arrayLength(&work_items)) {
        return;
    }
#endif

    // Unpack the work item.
    let input_index = work_items[instance_index].input_index;
#ifdef INDIRECT
    let indirect_parameters_index = work_items[instance_index].output_or_indirect_parameters_index;

    // If we're the first mesh instance in this batch, write the index of our
    // `MeshInput` into the appropriate slot so that the indirect parameters
    // building shader can access it.
#ifndef LATE_PHASE
    if (instance_index == 0u || work_items[instance_index - 1].output_or_indirect_parameters_index != indirect_parameters_index) {
        indirect_parameters_gpu_metadata[indirect_parameters_index].mesh_index = input_index;
    }
#endif  // LATE_PHASE

#else   // INDIRECT
    let mesh_output_index = work_items[instance_index].output_or_indirect_parameters_index;
#endif  // INDIRECT

    // Unpack the input matrix.
    let world_from_local_affine_transpose = current_input[input_index].world_from_local;
    let world_from_local = maths::affine3_to_square(world_from_local_affine_transpose);

    // Frustum cull if necessary.
#ifdef FRUSTUM_CULLING
    if ((current_input[input_index].flags & MESH_FLAGS_NO_FRUSTUM_CULLING_BIT) == 0u) {
        let aabb_center = mesh_culling_data[input_index].aabb_center.xyz;
        let aabb_half_extents = mesh_culling_data[input_index].aabb_half_extents.xyz;

        // Do an OBB-based frustum cull.
        let model_center = world_from_local * vec4(aabb_center, 1.0);
        if (!view_frustum_intersects_obb(world_from_local, model_center, aabb_half_extents)) {
            return;
        }
    }
#endif

    // See whether the `MeshInputUniform` was updated on this frame. If it
    // wasn't, then we know the transforms of this mesh must be identical to
    // those on the previous frame, and therefore we don't need to access the
    // `previous_input_index` (in fact, we can't; that index are only valid for
    // one frame and will be invalid).
    let timestamp = current_input[input_index].timestamp;
    let mesh_changed_this_frame = timestamp == view.frame_count;

    // Look up the previous model matrix, if it could have been.
    let previous_input_index = current_input[input_index].previous_input_index;
    var previous_world_from_local_affine_transpose: mat3x4<f32>;
    if (mesh_changed_this_frame && previous_input_index != 0xffffffffu) {
        previous_world_from_local_affine_transpose =
            previous_input[previous_input_index].world_from_local;
    } else {
        previous_world_from_local_affine_transpose = world_from_local_affine_transpose;
    }
    let previous_world_from_local =
        maths::affine3_to_square(previous_world_from_local_affine_transpose);

    // Occlusion cull if necessary. This is done by calculating the screen-space
    // axis-aligned bounding box (AABB) of the mesh and testing it against the
    // appropriate level of the depth pyramid (a.k.a. hierarchical Z-buffer). If
    // no part of the AABB is in front of the corresponding pixel quad in the
    // hierarchical Z-buffer, then this mesh must be occluded, and we can skip
    // rendering it.
#ifdef OCCLUSION_CULLING
    let aabb_center = mesh_culling_data[input_index].aabb_center.xyz;
    let aabb_half_extents = mesh_culling_data[input_index].aabb_half_extents.xyz;

    // Initialize the AABB and the maximum depth.
    let infinity = bitcast<f32>(0x7f800000u);
    let neg_infinity = bitcast<f32>(0xff800000u);
    var aabb = vec4(infinity, infinity, neg_infinity, neg_infinity);
    var max_depth_view = neg_infinity;

    // Build up the AABB by taking each corner of this mesh's OBB, transforming
    // it, and updating the AABB and depth accordingly.
    for (var i = 0u; i < 8u; i += 1u) {
        let local_pos = aabb_center + select(
            vec3(-1.0),
            vec3(1.0),
            vec3((i & 1) != 0, (i & 2) != 0, (i & 4) != 0)
        ) * aabb_half_extents;

#ifdef EARLY_PHASE
        // If we're in the early phase, we're testing against the last frame's
        // depth buffer, so we need to use the previous frame's transform.
        let prev_world_pos = (previous_world_from_local * vec4(local_pos, 1.0)).xyz;
        let view_pos = position_world_to_prev_view(prev_world_pos);
        let ndc_pos = position_world_to_prev_ndc(prev_world_pos);
#else   // EARLY_PHASE
        // Otherwise, if this is the late phase, we use the current frame's
        // transform.
        let world_pos = (world_from_local * vec4(local_pos, 1.0)).xyz;
        let view_pos = position_world_to_view(world_pos);
        let ndc_pos = position_world_to_ndc(world_pos);
#endif  // EARLY_PHASE

        let uv_pos = ndc_to_uv(ndc_pos.xy);

        // Update the AABB and maximum view-space depth.
        aabb = vec4(min(aabb.xy, uv_pos), max(aabb.zw, uv_pos));
        max_depth_view = max(max_depth_view, view_pos.z);
    }

    // Clip to the near plane to avoid the NDC depth becoming negative.
#ifdef EARLY_PHASE
    max_depth_view = min(-previous_view_uniforms.clip_from_view[3][2], max_depth_view);
#else   // EARLY_PHASE
    max_depth_view = min(-view.clip_from_view[3][2], max_depth_view);
#endif  // EARLY_PHASE

    // Figure out the depth of the occluder, and compare it to our own depth.

    let aabb_pixel_size = occlusion_culling::get_aabb_size_in_pixels(aabb, depth_pyramid);
    let occluder_depth_ndc =
        occlusion_culling::get_occluder_depth(aabb, aabb_pixel_size, depth_pyramid);

#ifdef EARLY_PHASE
    let max_depth_ndc = prev_view_z_to_depth_ndc(max_depth_view);
#else   // EARLY_PHASE
    let max_depth_ndc = view_z_to_depth_ndc(max_depth_view);
#endif

    // Are we culled out?
    if (max_depth_ndc < occluder_depth_ndc) {
#ifdef EARLY_PHASE
        // If this is the early phase, we need to make a note of this mesh so
        // that we examine it again in the late phase, so that we handle the
        // case in which a mesh that was invisible last frame became visible in
        // this frame.
        let output_work_item_index = atomicAdd(&late_preprocess_work_item_indirect_parameters[
            push_constants.late_preprocess_work_item_indirect_offset].work_item_count, 1u);
        if (output_work_item_index % 64u == 0u) {
            // Our workgroup size is 64, and the indirect parameters for the
            // late mesh preprocessing phase are counted in workgroups, so if
            // we're the first thread in this workgroup, bump the workgroup
            // count.
            atomicAdd(&late_preprocess_work_item_indirect_parameters[
                push_constants.late_preprocess_work_item_indirect_offset].dispatch_x, 1u);
        }

        // Enqueue a work item for the late prepass phase.
        late_preprocess_work_items[output_work_item_index].input_index = input_index;
        late_preprocess_work_items[output_work_item_index].output_or_indirect_parameters_index =
            indirect_parameters_index;
#endif  // EARLY_PHASE
        // This mesh is culled. Skip it.
        return;
    }
#endif  // OCCLUSION_CULLING

    // Calculate inverse transpose.
    let local_from_world_transpose = transpose(maths::inverse_affine3(transpose(
        world_from_local_affine_transpose)));

    // Pack inverse transpose.
    let local_from_world_transpose_a = mat2x4<f32>(
        vec4<f32>(local_from_world_transpose[0].xyz, local_from_world_transpose[1].x),
        vec4<f32>(local_from_world_transpose[1].yz, local_from_world_transpose[2].xy));
    let local_from_world_transpose_b = local_from_world_transpose[2].z;

    // Figure out the output index. In indirect mode, this involves bumping the
    // instance index in the indirect parameters metadata, which
    // `build_indirect_params.wgsl` will use to generate the actual indirect
    // parameters. Otherwise, this index was directly supplied to us.
#ifdef INDIRECT
#ifdef LATE_PHASE
    let batch_output_index = atomicLoad(
        &indirect_parameters_gpu_metadata[indirect_parameters_index].early_instance_count
    ) + atomicAdd(
        &indirect_parameters_gpu_metadata[indirect_parameters_index].late_instance_count,
        1u
    );
#else   // LATE_PHASE
    let batch_output_index = atomicAdd(
        &indirect_parameters_gpu_metadata[indirect_parameters_index].early_instance_count,
        1u
    );
#endif  // LATE_PHASE

    let mesh_output_index =
        indirect_parameters_cpu_metadata[indirect_parameters_index].base_output_index +
        batch_output_index;

#endif  // INDIRECT

    // Write the output.
    output[mesh_output_index].world_from_local = world_from_local_affine_transpose;
    output[mesh_output_index].previous_world_from_local =
        previous_world_from_local_affine_transpose;
    output[mesh_output_index].local_from_world_transpose_a = local_from_world_transpose_a;
    output[mesh_output_index].local_from_world_transpose_b = local_from_world_transpose_b;
    output[mesh_output_index].flags = current_input[input_index].flags;
    output[mesh_output_index].lightmap_uv_rect = current_input[input_index].lightmap_uv_rect;
    output[mesh_output_index].first_vertex_index = current_input[input_index].first_vertex_index;
    output[mesh_output_index].current_skin_index = current_input[input_index].current_skin_index;
    output[mesh_output_index].material_and_lightmap_bind_group_slot =
        current_input[input_index].material_and_lightmap_bind_group_slot;
    output[mesh_output_index].tag = current_input[input_index].tag;
}
