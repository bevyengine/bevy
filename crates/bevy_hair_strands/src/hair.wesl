// Forward vertex + fragment shader for hair strand ribbons.
//
// Shading is Kajiya-Kay with Scheuermann's two shifted specular lobes
// ("Practical Real-Time Hair Rendering and Shading", ATI 2004), lit by the
// same directional / point / spot lights and shadow maps as `StandardMaterial`.

import package::hair_functions::{material, HairVertex, decode_params, strand_width_scale, expand_ribbon};
import bevy_pbr::render::{
    mesh_functions,
    mesh_bindings::mesh,
    mesh_view_bindings as view_bindings,
    mesh_view_bindings::view,
    mesh_view_types,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    view_transformations::position_world_to_clip,
    clustered_forward as clustering,
    shadows,
    pbr_lighting::getDistanceAttenuation,
};
import bevy_render::maths::{PI, powsafe};

@if(DISTANCE_FOG)
import bevy_pbr::render::pbr_functions::apply_fog;

@if(TONEMAP_IN_SHADER)
import bevy_core_pipeline::tonemapping::{tone_mapping, screen_space_dither};

@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_tangent: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
    /// x: ribbon side (-1 / +1), y: root->tip parameter, z: per-strand seed.
    @location(3) params: vec3<f32>,
    @location(4) @interpolate(flat) instance_index: u32,
    @if(VISIBILITY_RANGE_DITHER)
    @location(5) @interpolate(flat) visibility_range_dither: i32,
}

@vertex
fn vertex(vertex: HairVertex) -> VertexOutput {
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let params = decode_params(vertex.params);
    let width_scale = strand_width_scale(world_from_local, params.z);
    let ribbon = expand_ribbon(
        world_from_local,
        vertex.position,
        vertex.tangent.xyz,
        params.x,
        params.y,
        width_scale,
    );

    var out: VertexOutput;
    out.world_position = ribbon.world_position;
    out.position = position_world_to_clip(ribbon.world_position.xyz);
    out.world_tangent = ribbon.world_tangent;
    out.world_normal = ribbon.world_normal;
    out.params = params;
    out.instance_index = vertex.instance_index;
@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index, world_from_local[3]);
    return out;
}

/// Bends the strand tangent along the normal so a highlight moves toward the
/// root (negative shift) or the tip (positive shift).
fn shift_tangent(T: vec3<f32>, N: vec3<f32>, shift: f32) -> vec3<f32> {
    return normalize(T + N * shift);
}

/// Kajiya-Kay anisotropic specular term for a strand with tangent `T`.
fn strand_specular(T: vec3<f32>, V: vec3<f32>, L: vec3<f32>, exponent: f32) -> f32 {
    let H = normalize(L + V);
    let dot_th = dot(T, H);
    let sin_th = sqrt(max(0.0, 1.0 - dot_th * dot_th));
    // Fade the lobe out on the far side of the strand.
    let dir_atten = smoothstep(-1.0, 0.0, dot_th);
    return dir_atten * pow(sin_th, exponent);
}

/// Everything about the surface that is the same for every light.
struct HairSurface {
    albedo: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    /// Tangent shifted for the primary highlight.
    T1: vec3<f32>,
    /// Tangent shifted for the secondary highlight.
    T2: vec3<f32>,
    /// Per-strand modulation of the secondary highlight so it breaks up.
    secondary_mask: f32,
}

/// Outgoing radiance for one light with unit direction `L` (towards the light).
fn shade_light(surface: HairSurface, L: vec3<f32>) -> vec3<f32> {
    // Wrapped Lambert: strands are lit a little even when the ribbon normal
    // faces away, because real hair scatters light through neighbouring strands.
    let wrap = saturate(mix(0.25, 1.0, dot(surface.N, L)));
    let diffuse = surface.albedo * wrap * (1.0 / PI);

    let primary = strand_specular(surface.T1, surface.V, L, material.specular_exponent)
        * material.specular_color.rgb;
    let secondary = strand_specular(surface.T2, surface.V, L, material.secondary_exponent)
        * material.secondary_specular_color.rgb * surface.secondary_mask;

    // Specular energy should not appear when the light is fully behind the strand.
    let visibility = saturate(dot(surface.N, L) * 4.0 + 0.5);
    return diffuse + (primary + secondary) * visibility;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    // In the crossfade section of a visibility range, discard by the pattern.
@if(VISIBILITY_RANGE_DITHER)
    visibility_range_dither(in.position, in.visibility_range_dither);

    let seed = in.params.z;
    let t = in.params.y;

    // Ribbons are double sided; flip the shading normal for the back face so
    // both sides light identically.
    var N = normalize(in.world_normal);
    if !is_front {
        N = -N;
    }
    let T = normalize(in.world_tangent);

    let is_orthographic = view.clip_from_view[3][3] == 1.0;
    var V: vec3<f32>;
    if is_orthographic {
        V = normalize(vec3(view.clip_from_world[0].z, view.clip_from_world[1].z, view.clip_from_world[2].z));
    } else {
        V = normalize(view.world_position - in.world_position.xyz);
    }

    // Break the highlights up per strand so they do not read as one solid band.
    let shift_noise = (seed - 0.5) * 0.2;
    let brightness = 1.0 + (seed * 2.0 - 1.0) * material.color_variation;
    let root_ao = mix(material.root_occlusion, 1.0, t);

    var surface: HairSurface;
    surface.albedo = material.base_color.rgb * brightness * root_ao;
    surface.N = N;
    surface.V = V;
    surface.T1 = shift_tangent(T, N, material.specular_shift + shift_noise);
    surface.T2 = shift_tangent(T, N, material.secondary_shift + shift_noise);
    surface.secondary_mask = 0.5 + 0.5 * fract(seed * 7.31);

    let mesh_flags = mesh[in.instance_index].flags;
    let receives_shadows = (mesh_flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u;

    // Cluster lookup for the point/spot lights affecting this fragment.
    let view_z = dot(vec4<f32>(
        view.view_from_world[0].z,
        view.view_from_world[1].z,
        view.view_from_world[2].z,
        view.view_from_world[3].z
    ), in.world_position);
    let cluster_index = clustering::view_fragment_cluster_index(in.position.xy, view_z, is_orthographic);
    let ranges = clustering::unpack_clusterable_object_index_ranges(cluster_index);

    var color = vec3(0.0);

    // Point lights.
    for (var i: u32 = ranges.first_point_light_index_offset;
            i < ranges.first_spot_light_index_offset;
            i = i + 1u) {
        let light_id = clustering::get_clusterable_object_id(i);
        let light = &view_bindings::clustered_lights.data[light_id];
        let light_to_frag = (*light).position_radius.xyz - in.world_position.xyz;
        let distance_square = dot(light_to_frag, light_to_frag);
        let L = light_to_frag * inverseSqrt(max(distance_square, 1e-8));
        let attenuation = getDistanceAttenuation(distance_square, (*light).color_inverse_square_range.w);

        var shadow = 1.0;
        if receives_shadows
                && ((*light).flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = shadows::fetch_point_shadow(light_id, in.world_position, N, in.position.xy);
        }

        color += shade_light(surface, L) * (*light).color_inverse_square_range.rgb * attenuation * shadow;
    }

    // Spot lights.
    for (var i: u32 = ranges.first_spot_light_index_offset;
            i < ranges.first_rect_light_index_offset;
            i = i + 1u) {
        let light_id = clustering::get_clusterable_object_id(i);
        let light = &view_bindings::clustered_lights.data[light_id];
        let light_to_frag = (*light).position_radius.xyz - in.world_position.xyz;
        let distance_square = dot(light_to_frag, light_to_frag);
        let L = light_to_frag * inverseSqrt(max(distance_square, 1e-8));
        let range_attenuation = getDistanceAttenuation(distance_square, (*light).color_inverse_square_range.w);

        // Reconstruct the cone direction and apply the precomputed cone falloff.
        var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
        spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
        if ((*light).flags & mesh_view_types::POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u {
            spot_dir.y = -spot_dir.y;
        }
        let cd = dot(-spot_dir, L);
        let cone = saturate(cd * (*light).light_custom_data.z + (*light).light_custom_data.w);
        let spot_attenuation = cone * cone;

        var shadow = 1.0;
        if receives_shadows
                && ((*light).flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = shadows::fetch_spot_shadow(
                light_id,
                in.world_position,
                N,
                (*light).shadow_map_near_z,
                in.position.xy,
            );
        }

        color += shade_light(surface, L) * (*light).color_inverse_square_range.rgb
            * range_attenuation * spot_attenuation * shadow;
    }

    // Directional lights.
    let n_directional_lights = view_bindings::lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
        let light = &view_bindings::lights.directional_lights[i];
        let L = (*light).direction_to_light;

        var shadow = 1.0;
        if receives_shadows
                && ((*light).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = shadows::fetch_directional_shadow(i, in.world_position, N, view_z, in.position.xy);
        }

        color += shade_light(surface, L) * (*light).color.rgb * shadow;
    }

    // Ambient.
    color += surface.albedo * view_bindings::lights.ambient_color.rgb;

    var output_color = vec4(color * view.exposure, 1.0);

@if(DISTANCE_FOG)
    output_color = apply_fog(
        view_bindings::fog,
        output_color,
        in.world_position.xyz,
        view.world_position,
        in.position.xy,
    );

@if(TONEMAP_IN_SHADER) {
    output_color = tone_mapping(output_color, view.color_grading);
@if(DEBAND_DITHER) {
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb += screen_space_dither(in.position.xy);
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
}
}

    return output_color;
}
