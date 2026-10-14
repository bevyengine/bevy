import package::atmosphere::{
    bindings::{view, settings, cloud_shadow_map, atmosphere_lut_sampler},
    bruneton_functions::sample_transmittance_lut_segment,
    functions::{
        sample_transmittance_lut,
        sample_sky_view_lut, direction_world_to_atmosphere,
        uv_to_ray_direction, uv_to_ndc, sample_aerial_view_lut,
        sample_sun_radiance, ndc_to_camera_dist, raymarch_atmosphere,
        get_view_position, max_atmosphere_distance
    },
};

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

@if(CLOUDS_ENABLED)
import package::atmosphere::{bindings::lights, functions::sample_cloud_shadow_map};


@if(DUAL_SOURCE_BLENDING)
enable dual_source_blending;

@if(MULTISAMPLED)
@group(0) @binding(13) var depth_texture: texture_depth_multisampled_2d;
@else
@group(0) @binding(13) var depth_texture: texture_depth_2d;

@if(NO_CUBE_ARRAY_TEXTURES_SUPPORT)
@group(0) @binding(14) var point_shadow_textures: texture_depth_cube;
@else
@group(0) @binding(14) var point_shadow_textures: texture_depth_cube_array;
@group(0) @binding(15) var point_shadow_textures_comparison_sampler: sampler_comparison;

// Output of the low-res `clouds_raymarch` pass: rgb = pre-exposed cloud luminance, a = cloud
// transmittance; and the transmittance-weighted cloud distance (m).
@if(CLOUDS_ENABLED)
@group(0) @binding(26) var cloud_texture: texture_2d<f32>;
@if(CLOUDS_ENABLED)
@group(0) @binding(27) var cloud_depth_texture: texture_2d<f32>;

struct RenderSkyOutput {
    @if(DUAL_SOURCE_BLENDING)
    @location(0) @blend_src(0) inscattering: vec4<f32>,
    @if(DUAL_SOURCE_BLENDING)
    @location(0) @blend_src(1) transmittance: vec4<f32>,
    @else
    @location(0) inscattering: vec4<f32>,
}

// Aerial perspective over [0, d] with cloud volumetric shadowing (RDR2 / Bauer 2019: the cloud
// shadow map shadows atmosphere in-scattering). The aerial-view LUT is unshadowed, so split the
// segment into N pieces along the LUT's squared depth distribution and scale each piece's
// in-scatter increment by the cloud-shadow transmittance (sun light 0) at its midpoint.
@if(CLOUDS_ENABLED)
fn cloud_shadowed_aerial_perspective(uv: vec2<f32>, origin: vec3<f32>, ray_dir: vec3<f32>, d: f32) -> vec3<f32> {
    if (lights.n_directional_lights == 0u) {
        return sample_aerial_view_lut(uv, d);
    }
    let l = lights.directional_lights[0].direction_to_light;
    const N: u32 = 8u;
    var prev = vec3(0.0);
    var acc = vec3(0.0);
    for (var i = 1u; i <= N; i++) {
        let x = f32(i) / f32(N);
        let xm = (f32(i) - 0.5) / f32(N);
        let cur = sample_aerial_view_lut(uv, d * x * x);
        let vis = sample_cloud_shadow_map(origin + ray_dir * (d * xm * xm), l, false);
        acc += max(cur - prev, vec3(0.0)) * vis;
        prev = cur;
    }
    return acc;
}

@fragment
fn main(in: FullscreenVertexOutput) -> RenderSkyOutput {
    let depth = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);

    let ray_dir_ws = uv_to_ray_direction(in.uv);
    let world_pos = get_view_position();
    let r = length(world_pos);
    let up = normalize(world_pos);
    let mu = dot(ray_dir_ws, up);
    let max_samples = settings.sky_max_samples;
    let should_raymarch = settings.rendering_method == 1u;

    var transmittance: vec3<f32>;
    var inscattering: vec3<f32>;

    let sun_radiance = sample_sun_radiance(ray_dir_ws);

    if depth == 0.0 {
        let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws);
        transmittance = sample_transmittance_lut(r, mu);
        inscattering = sample_sky_view_lut(r, ray_dir_as);
        if should_raymarch {
            let t_max = max_atmosphere_distance(r, mu);
            let result = raymarch_atmosphere(world_pos, ray_dir_ws, t_max, max_samples, in.uv, true);
            inscattering = result.inscattering;
            transmittance = result.transmittance;
        }
        inscattering += sun_radiance * transmittance;
    } else {
        let t = ndc_to_camera_dist(vec3(uv_to_ndc(in.uv), depth));
        inscattering = sample_aerial_view_lut(in.uv, t);
        transmittance = sample_transmittance_lut_segment(r, mu, t);
        if should_raymarch {
            let result = raymarch_atmosphere(world_pos, ray_dir_ws, t, max_samples, in.uv, false);
            inscattering = result.inscattering;
            transmittance = result.transmittance;
        }
    }

    // exposure compensation
    inscattering *= view.exposure;

    @if(CLOUDS_ENABLED) {
    // Composite the low-res cloud pass with a depth-aware bilinear upsample (roadmap 1e, first
    // part): each of the 4 bilinear taps only contributes if its cloud lies in front of this
    // full-res pixel's scene depth, then the weights are renormalized. With the cloud at
    // weighted distance d, transmittance T_c and luminance L_c:
    //   L = L_behind * T_c + (1 - T_c) * L_ap(0..d) + T_ap(0..d) * L_c
    // which assumes the atmosphere in-scattering behind the cloud is ~ the full-ray value.
    var scene_t = 1e30;
    if depth != 0.0 {
        scene_t = ndc_to_camera_dist(vec3(uv_to_ndc(in.uv), depth));
    }
    let cloud_dims_u = textureDimensions(cloud_texture);
    let cloud_dims = vec2<f32>(cloud_dims_u);
    let cloud_coord = in.uv * cloud_dims - 0.5;
    let cloud_base = vec2<i32>(floor(cloud_coord));
    let cloud_frac = cloud_coord - floor(cloud_coord);
    var clouds = vec4(0.0);
    var cloud_t = 0.0;
    var cloud_w_sum = 0.0;
    for (var tap = 0; tap < 4; tap++) {
        let o = vec2(tap & 1, tap >> 1);
        let p = clamp(cloud_base + o, vec2(0), vec2<i32>(cloud_dims_u) - 1);
        let bw = select(1.0 - cloud_frac.x, cloud_frac.x, o.x == 1)
            * select(1.0 - cloud_frac.y, cloud_frac.y, o.y == 1);
        let c = textureLoad(cloud_texture, p, 0);
        let d = textureLoad(cloud_depth_texture, p, 0).r;
        // Cloud taps behind the scene (upsampling across a silhouette) count as clear sky.
        let valid = d < scene_t;
        let c_eff = select(vec4(0.0, 0.0, 0.0, 1.0), c, valid);
        clouds += c_eff * bw;
        let opacity = 1.0 - c_eff.a;
        cloud_t += d * bw * opacity;
        cloud_w_sum += bw * opacity;
    }
    cloud_t = cloud_t / max(cloud_w_sum, 1e-6);
    if clouds.a < 0.999 && cloud_t > 0.0 && cloud_t < scene_t {
        let t_c = clouds.a;
        let transmittance_to_cloud = sample_transmittance_lut_segment(r, mu, cloud_t);
        var inscattering_to_cloud = cloud_shadowed_aerial_perspective(in.uv, world_pos, ray_dir_ws, cloud_t) * view.exposure;
        if depth == 0.0 {
            // Towards and beyond the end of the aerial-view LUT range, fade to assuming the sky
            // in-scattering is distributed along the ray like the optical depth (no hard seam).
            let ap_max = settings.aerial_view_lut_max_distance;
            let w = smoothstep(0.5 * ap_max, ap_max, cloud_t);
            if w > 0.0 {
                let sky_inscattering = inscattering - sun_radiance * transmittance * view.exposure;
                let frac = (1.0 - transmittance_to_cloud)
                    / max(vec3(1.0) - transmittance, vec3(1e-4));
                inscattering_to_cloud =
                    mix(inscattering_to_cloud, sky_inscattering * saturate(frac), w);
            }
        }
        inscattering = inscattering * t_c + (1.0 - t_c) * inscattering_to_cloud
            + transmittance_to_cloud * clouds.rgb;
        transmittance *= t_c;
    }
    }
    
    @if(DUAL_SOURCE_BLENDING)
    return RenderSkyOutput(vec4(inscattering, 0.0), vec4(transmittance, 1.0));
    @if(!DUAL_SOURCE_BLENDING)
    let mean_transmittance = (transmittance.r + transmittance.g + transmittance.b) / 3.0;
    @if(!DUAL_SOURCE_BLENDING)
    return RenderSkyOutput(vec4(inscattering, mean_transmittance));
    
}
