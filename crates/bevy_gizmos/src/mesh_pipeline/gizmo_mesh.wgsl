#import bevy_render::{maths::{affine_to_square, mat2x4_f32_to_mat3x3_unpack}, instance_index::get_instance_index, view::{View, coords_to_ray_direction}}
#import bevy_core_pipeline::tonemapping::{screen_space_dither, powsafe, tone_mapping}
// #import bevy_gizmos::utils::calculate_depth

@group(0) @binding(0) var<uniform> view: View;

struct Gizmo {
    // Affine 4x3 matrices transposed to 3x4
    // Use bevy_render::maths::affine_to_square to unpack
    transform: mat3x4<f32>,
    // 3x3 matrix packed in mat2x4 and f32 as:
    // [0].xyz, [1].x,
    // [1].yz, [2].xy
    // [2].z
    // Use bevy_render::maths::mat2x4_f32_to_mat3x3_unpack to unpack
    inverse_transpose_transform_a: mat2x4<f32>,
    inverse_transpose_transform_b: f32,
    color: vec4<f32>
};

#ifdef PER_OBJECT_BUFFER_BATCH_SIZE
@group(1) @binding(0) var<uniform> gizmos: array<Gizmo, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
#else
@group(1) @binding(0) var<storage> gizmos: array<Gizmo>;
#endif

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let gizmo = gizmos[in.instance_index];

    let transform = affine_to_square(gizmo.transform);
    out.position = view.view_proj * transform * vec4(in.position, 1.0);

#ifdef VERTEX_COLORS
    out.color = in.color * gizmo.color;
#elseif
    out.color = gizmo.color;
#endif

#ifdef VERTEX_NORMALS
    let inverse_transform = mat2x4_f32_to_mat3x3_unpack(
        gizmo.inverse_transpose_transform_a,
        gizmo.inverse_transpose_transform_b,
    );
    out.world_normal = normalize(inverse_transform * in.normal);
#endif

    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    var color = in.color;

#ifdef VERTEX_NORMALS
    #ifdef 3D 
        let view_direction = coords_to_ray_direction(in.position.xy, view);
    #elseif
        let view_direction = vec3(0., 0., -1.);
    #endif
    // Fake lighting
    let d = dot(view_direction, in.world_normal);
    color = mix(color, vec4(vec3(0.), 1.), d);
#endif

// TODO: We don't need tonemapping in this shader, but screen_space_dither would be nice to have.
// The fake lighting above also doesn't work without tonemapping for some reason.

#ifdef TONEMAP_IN_SHADER
    color = tone_mapping(color, view.color_grading);
#ifdef DEBAND_DITHER
    var color_rgb = color.rgb;

    // Convert to sRGB
    color_rgb = powsafe(color_rgb, 1.0 / 2.2);
    color_rgb += screen_space_dither(in.position.xy);
    // Convert back to Linear sRGB
    color_rgb = powsafe(color_rgb, 2.2);

    color = vec4(color_rgb, color.a);
#endif
#endif

    out.color = color;

    return out;
}
