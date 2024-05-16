// A postprocessing shader that implements volumetric fog via raymarching and
// sampling directional light shadow maps.
//
// The overall approach is a combination of the volumetric rendering in [1] and
// the shadow map raymarching in [2]. First, we sample the depth buffer to
// determine how long our ray is. Then we do a raymarch, with physically-based
// calculations at each step to determine how much light was absorbed, scattered
// out, and scattered in. To determine in-scattering, we sample the shadow map
// for the light to determine whether the point was in shadow or not.
//
// [1]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/intro-volume-rendering.html
//
// [2]: http://www.alexandre-pestana.com/volumetric-lights/

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::mesh_view_bindings::{lights, view}
#import bevy_pbr::mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_VOLUMETRIC_BIT
#import bevy_pbr::shadow_sampling::sample_shadow_map_hardware
#import bevy_pbr::shadows::{get_cascade_index, world_to_directional_light_local}
#import bevy_pbr::view_transformations::{
    frag_coord_to_ndc,
    position_ndc_to_view,
    position_ndc_to_world
}

// The GPU version of [`VolumetricFogSettings`]. See the comments in
// `volumetric_fog/mod.rs` for descriptions of the fields here.
struct VolumetricFog {
    fog_color: vec3<f32>,
    light_tint: vec3<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    step_count: u32,
    max_depth: f32,
    absorption: f32,
    scattering: f32,
    density: f32,
    scattering_asymmetry: f32,
    light_intensity: f32,
}

@group(1) @binding(0) var<uniform> volumetric_fog: VolumetricFog;
@group(1) @binding(1) var color_texture: texture_2d<f32>;
@group(1) @binding(2) var color_sampler: sampler;

#ifdef MULTISAMPLED
@group(1) @binding(3) var depth_texture: texture_depth_multisampled_2d;
#else
@group(1) @binding(3) var depth_texture: texture_depth_2d;
#endif

// 1 / (4π)
const FRAC_4_PI: f32 = 0.07957747154594767;

// The common Henyey-Greenstein asymmetric phase function [1] [2].
//
// This determines how much light goes toward the viewer as opposed to away from
// the viewer. From a visual point of view, it controls how the light shafts
// appear and disappear as the camera looks at the light source.
//
// [1]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/ray-marching-get-it-right.html
//
// [2]: https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions#TheHenyeyndashGreensteinPhaseFunction
fn henyey_greenstein(neg_LdotV: f32) -> f32 {
    let g = volumetric_fog.scattering_asymmetry;
    let denom = 1.0 + g * g - 2.0 * g * neg_LdotV;
    return FRAC_4_PI * (1.0 - g * g) / (denom * sqrt(denom));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Unpack the `volumetric_fog` settings.
    let fog_color = volumetric_fog.fog_color;
    let ambient_color = volumetric_fog.ambient_color;
    let ambient_intensity = volumetric_fog.ambient_intensity;
    let step_count = volumetric_fog.step_count;
    let max_depth = volumetric_fog.max_depth;
    let absorption = volumetric_fog.absorption;
    let scattering = volumetric_fog.scattering;
    let density = volumetric_fog.density;
    let light_tint = volumetric_fog.light_tint;
    let light_intensity = volumetric_fog.light_intensity;

    let exposure = view.exposure;

    // Sample the depth. If this is multisample, just use sample 0; this is
    // approximate but good enough.
    let frag_coord = in.position;
    let depth = textureLoad(depth_texture, vec2<i32>(frag_coord.xy), 0);

    // Starting at the end depth, which we got above, figure out how long the
    // ray we want to trace is and the length of each increment.
    let end_depth = min(
        max_depth,
        -position_ndc_to_view(frag_coord_to_ndc(vec4(in.position.xy, depth, 1.0))).z
    );
    let step_size = end_depth / f32(step_count);

    let directional_light_count = lights.n_directional_lights;

    // Calculate the ray origin (`Ro`) and the ray direction (`Rd`) in NDC,
    // view, and world coordinates.
    let Rd_ndc = vec3(frag_coord_to_ndc(in.position).xy, 1.0);
    let Rd_view = normalize(position_ndc_to_view(Rd_ndc));
    let Ro_world = view.world_position;
    let Rd_world = normalize(position_ndc_to_world(Rd_ndc) - Ro_world);

    // Use Beer's law [1] [2] to calculate the maximum amount of light that each
    // directional light could contribute, and modulate that value by the light
    // tint and fog color. (The actual value will in turn be modulated by the
    // phase according to the Henyey-Greenstein formula.)
    //
    // We use a bit of a hack here. Conceptually, directional lights are
    // infinitely far away. But, if we modeled exactly that, then directional
    // lights would never contribute any light to the fog, because an
    // infinitely-far directional light combined with an infinite amount of fog
    // would result in complete absorption of the light. So instead we pretend
    // that the directional light is `max_depth` units away and do the
    // calculation in those terms. Because the fake distance to the directional
    // light is a constant, this lets us perform the calculation once up here
    // instead of marching secondary rays toward the light during the
    // raymarching step, which improves performance dramatically.
    //
    // [1]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/intro-volume-rendering.html
    //
    // [2]: https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law
    let light_attenuation = exp(-density * max_depth * (absorption + scattering));
    let light_factors_per_step = fog_color * light_tint * light_attenuation * scattering *
        density * step_size * light_intensity * exposure;

    // Use Beer's law again to accumulate the ambient light all along the path.
    var accumulated_color = exp(-end_depth * (absorption + scattering)) * ambient_color *
        ambient_intensity;

    // Pre-calculate absorption (amount of light absorbed by the fog) and
    // out-scattering (amount of light the fog scattered away). This is the same
    // amount for every step.
    let sample_attenuation = exp(-step_size * density * (absorption + scattering));

    // This is the amount of the background that shows through. We're actually
    // going to recompute this over and over again for each directional light,
    // coming up with the same values each time.
    var background_alpha = 1.0;

    for (var light_index = 0u; light_index < directional_light_count; light_index += 1u) {
        // Volumetric lights are all sorted first, so the first time we come to
        // a non-volumetric light, we know we've seen them all.
        let light = &lights.directional_lights[light_index];
        if (((*light).flags & DIRECTIONAL_LIGHT_FLAGS_VOLUMETRIC_BIT) == 0) {
            break;
        }

        // Offset the depth value by the bias.
        let depth_offset = (*light).shadow_depth_bias * (*light).direction_to_light.xyz;

        // Compute phase, which determines the fraction of light that's
        // scattered toward the camera instead of away from it.
        let neg_LdotV = dot(normalize((*light).direction_to_light.xyz), Rd_world);
        let phase = henyey_greenstein(neg_LdotV);

        // Modulate the factor we calculated above by the phase, fog color,
        // light color, light tint.
        let light_color_per_step = (*light).color.rgb * phase * light_factors_per_step;

        // Reset `background_alpha` for a new raymarch.
        background_alpha = 1.0;

        // Start raymarching.
        for (var step = 0u; step < step_count; step += 1u) {
            // As an optimization, break if we've gotten too dark.
            if (background_alpha < 0.001) {
                break;
            }

            // Calculate where we are in the ray.
            let P_world = Ro_world + Rd_world * f32(step) * step_size;
            let P_view = Rd_view * f32(step) * step_size;

            // Process absorption and out-scattering.
            background_alpha *= sample_attenuation;

            // Compute in-scattering (amount of light other fog particles
            // scattered into this ray). This is where any directional light is
            // scattered in.

            // Prepare to sample the shadow map.
            let cascade_index = get_cascade_index(light_index, P_view.z);
            let light_local = world_to_directional_light_local(
                light_index,
                cascade_index,
                vec4(P_world + depth_offset, 1.0)
            );

            // If we're outside the shadow map entirely, local light attenuation
            // is zero.
            var local_light_attenuation = f32(light_local.w != 0.0);

            // Otherwise, sample the shadow map to determine whether, and by how
            // much, this sample is in the light.
            if (local_light_attenuation != 0.0) {
                let cascade = &(*light).cascades[cascade_index];
                let array_index = i32((*light).depth_texture_base_index + cascade_index);
                local_light_attenuation =
                    sample_shadow_map_hardware(light_local.xy, light_local.z, array_index);
            }

            if (local_light_attenuation != 0.0) {
                // Accumulate the light.
                accumulated_color += light_color_per_step * local_light_attenuation *
                    background_alpha;
            }
        }
    }

    // We're done! Blend between the source color and the lit fog color.
    let source = textureSample(color_texture, color_sampler, in.uv);
    return vec4(source.rgb * background_alpha + accumulated_color, source.a);
}
