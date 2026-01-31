#define_import_path bevy_pbr::prepass_io

// Most of these attributes are not used in the default prepass fragment shader, but they are still needed so we can
// pass them to custom prepass shaders like pbr_prepass.wgsl.
struct UncompressedVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

#ifdef VERTEX_UVS_A
    @location(1) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_B
    @location(2) uv_b: vec2<f32>,
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
    @location(3) normal: vec3<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif

#ifdef VERTEX_COLORS
    @location(7) color: vec4<f32>,
#endif

#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif // MORPH_TARGETS
};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS_COMPRESSED
    @location(0) compressed_position: vec4<f32>,
#else
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_UVS_A
#ifdef VERTEX_UVS_A_COMPRESSED
    @location(1) compressed_uv: vec2<f32>,
#else
    @location(1) uv: vec2<f32>,
#endif
#endif
#ifdef VERTEX_UVS_B
#ifdef VERTEX_UVS_B_COMPRESSED
    @location(2) compressed_uv_b: vec2<f32>,
#else
    @location(2) uv_b: vec2<f32>,
#endif
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
#ifdef VERTEX_NORMALS_COMPRESSED
    @location(3) compressed_normal: vec2<f32>,
#else
    @location(3) normal: vec3<f32>,
#endif
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    @location(4) compressed_tangent: vec2<f32>,
#else
    @location(4) tangent: vec4<f32>,
#endif
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif

#ifdef VERTEX_COLORS
    @location(7) color: vec4<f32>,
#endif

#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif // MORPH_TARGETS
}

fn decompress_vertex(vertex_in: Vertex) -> UncompressedVertex {
    var uncompressed_vertex: UncompressedVertex;
    uncompressed_vertex.instance_index = vertex_in.instance_index;
#ifdef VERTEX_POSITIONS_COMPRESSED
    uncompressed_vertex.position = bevy_pbr::mesh_functions::decompress_vertex_position(vertex_in.instance_index, vertex_in.compressed_position);
#else
    uncompressed_vertex.position = vertex_in.position;
#endif
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
#ifdef VERTEX_NORMALS_COMPRESSED
    uncompressed_vertex.normal = bevy_pbr::mesh_functions::decompress_vertex_normal(vertex_in.compressed_normal);
#else
    uncompressed_vertex.normal = vertex_in.normal;
#endif
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    uncompressed_vertex.tangent = bevy_pbr::mesh_functions::decompress_vertex_tangent(vertex_in.compressed_tangent);
#else
    uncompressed_vertex.tangent = vertex_in.tangent;
#endif
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_UVS_A
#ifdef VERTEX_UVS_A_COMPRESSED
    uncompressed_vertex.uv = bevy_pbr::mesh_functions::decompress_vertex_uv(vertex_in.instance_index, vertex_in.compressed_uv);
#else
    uncompressed_vertex.uv = vertex_in.uv;
#endif
#endif
#ifdef VERTEX_UVS_B
#ifdef VERTEX_UVS_B_COMPRESSED
    uncompressed_vertex.uv_b = bevy_pbr::mesh_functions::decompress_vertex_uv_b(vertex_in.instance_index, vertex_in.compressed_uv_b);
#else
    uncompressed_vertex.uv_b = vertex_in.uv_b;
#endif
#endif
#ifdef VERTEX_COLORS
    uncompressed_vertex.color = vertex_in.color;
#endif
#ifdef SKINNED
    uncompressed_vertex.joint_indices = vertex_in.joint_indices;
    uncompressed_vertex.joint_weights = vertex_in.joint_weights;
#endif
#ifdef MORPH_TARGETS
    uncompressed_vertex.index = vertex_in.index;
#endif
    return uncompressed_vertex;
}

struct VertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,

#ifdef VERTEX_UVS_A
    @location(0) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_B
    @location(1) uv_b: vec2<f32>,
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(2) world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

    @location(4) world_position: vec4<f32>,
#ifdef MOTION_VECTOR_PREPASS
    @location(5) previous_world_position: vec4<f32>,
#endif

#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    @location(6) unclipped_depth: f32,
#endif // UNCLIPPED_DEPTH_ORTHO_EMULATION
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(7) instance_index: u32,
#endif

#ifdef VERTEX_COLORS
    @location(8) color: vec4<f32>,
#endif

#ifdef VISIBILITY_RANGE_DITHER
    @location(9) @interpolate(flat) visibility_range_dither: i32,
#endif  // VISIBILITY_RANGE_DITHER
}

#ifdef PREPASS_FRAGMENT
struct FragmentOutput {
#ifdef NORMAL_PREPASS
    @location(0) normal: vec4<f32>,
#endif

#ifdef MOTION_VECTOR_PREPASS
    @location(1) motion_vector: vec2<f32>,
#endif

#ifdef DEFERRED_PREPASS
    @location(2) deferred: vec4<u32>,
    @location(3) deferred_lighting_pass_id: u32,
#endif

#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    @builtin(frag_depth) frag_depth: f32,
#endif // UNCLIPPED_DEPTH_ORTHO_EMULATION
}
#endif //PREPASS_FRAGMENT
