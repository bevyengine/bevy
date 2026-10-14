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
    // Composite the low-res cloud pass (bilinear upsample for now; roadmap 1e makes it
    // depth aware). With the cloud at weighted distance d, transmittance T_c and luminance L_c:
    //   L = L_behind * T_c + (1 - T_c) * L_ap(0..d) + T_ap(0..d) * L_c
    // which assumes the atmosphere in-scattering behind the cloud is ~ the full-ray value.
    let clouds = textureSampleLevel(cloud_texture, atmosphere_lut_sampler, in.uv, 0.0);
    let cloud_dims = vec2<f32>(textureDimensions(cloud_depth_texture));
    let cloud_px = vec2<i32>(min(in.uv * cloud_dims, cloud_dims - 1.0));
    let cloud_t = textureLoad(cloud_depth_texture, cloud_px, 0).r;
    var scene_t = 1e30;
    if depth != 0.0 {
        scene_t = ndc_to_camera_dist(vec3(uv_to_ndc(in.uv), depth));
    }
    // Reject clouds behind the full-res scene depth (upsampling across silhouettes).
    if clouds.a < 0.999 && cloud_t > 0.0 && cloud_t < scene_t {
        let t_c = clouds.a;
        let transmittance_to_cloud = sample_transmittance_lut_segment(r, mu, cloud_t);
        var inscattering_to_cloud: vec3<f32>;
        if depth == 0.0 && cloud_t > settings.aerial_view_lut_max_distance {
            // Beyond the aerial-view LUT range: assume the in-scattering is distributed along
            // the ray like the optical depth.
            let sky_inscattering = inscattering - sun_radiance * transmittance * view.exposure;
            let frac = (1.0 - transmittance_to_cloud) / max(vec3(1.0) - transmittance, vec3(1e-4));
            inscattering_to_cloud = sky_inscattering * saturate(frac);
        } else {
            inscattering_to_cloud = sample_aerial_view_lut(in.uv, cloud_t) * view.exposure;
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
