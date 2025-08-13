// A postprocessing shader that implements volumetric fog via raymarching and
// sampling directional light shadow maps.
//
// The overall approach is a combination of the volumetric rendering in [1] and
// the shadow map raymarching in [2]. First, we raytrace the AABB of the fog
// volume in order to determine how long our ray is. Then we do a raymarch, with
// physically-based calculations at each step to determine how much light was
// absorbed, scattered out, and scattered in. To determine in-scattering, we
// sample the shadow map for the light to determine whether the point was in
// shadow or not.
//
// [1]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/intro-volume-rendering.html
//
// [2]: http://www.alexandre-pestana.com/volumetric-lights/

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::{globals, lights, view, clusterable_objects}
#import bevy_pbr::mesh_view_types::{
    DIRECTIONAL_LIGHT_FLAGS_VOLUMETRIC_BIT, 
    POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT, 
    POINT_LIGHT_FLAGS_VOLUMETRIC_BIT,
    POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE,
    ClusterableObject
}
#import bevy_pbr::shadow_sampling::{
    sample_shadow_map_hardware, 
    sample_shadow_cubemap,
    sample_shadow_map,
    SPOT_SHADOW_TEXEL_SIZE
}
#import bevy_pbr::shadows::{get_cascade_index, world_to_directional_light_local}
#import bevy_pbr::utils::interleaved_gradient_noise
#import bevy_pbr::view_transformations::{
    depth_ndc_to_view_z,
    frag_coord_to_ndc,
    position_ndc_to_view,
    position_ndc_to_world,
    position_view_to_world
}
#import bevy_pbr::clustered_forward as clustering
#import bevy_pbr::lighting::getDistanceAttenuation;

// The GPU version of [`VolumetricFog`]. See the comments in
// `volumetric_fog/mod.rs` for descriptions of the fields here.
struct VolumetricFog {
    clip_from_local: mat4x4<f32>,
    uvw_from_world: mat4x4<f32>,
    far_planes: array<vec4<f32>, 3>,
    fog_color: vec3<f32>,
    light_tint: vec3<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    step_count: u32,
    bounding_radius: f32,
    absorption: f32,
    scattering: f32,
    density_factor: f32,
    density_texture_offset: vec3<f32>,
    scattering_asymmetry: f32,
    light_intensity: f32,
    jitter_strength: f32,
}

@group(1) @binding(0) var<uniform> volumetric_fog: VolumetricFog;

#ifdef MULTISAMPLED
@group(1) @binding(1) var depth_texture: texture_depth_multisampled_2d;
#else
@group(1) @binding(1) var depth_texture: texture_depth_2d;
#endif

#ifdef DENSITY_TEXTURE
@group(1) @binding(2) var density_texture: texture_3d<f32>;
@group(1) @binding(3) var density_sampler: sampler;
#endif  // DENSITY_TEXTURE

// 1 / (4π)
const FRAC_4_PI: f32 = 0.07957747154594767;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> @builtin(position) vec4<f32> {
    return volumetric_fog.clip_from_local * vec4<f32>(vertex.position, 1.0);
}

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
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // Unpack the `volumetric_fog` settings.
    let uvw_from_world = volumetric_fog.uvw_from_world;
    let fog_color = volumetric_fog.fog_color;
    let ambient_color = volumetric_fog.ambient_color;
    let ambient_intensity = volumetric_fog.ambient_intensity;
    let step_count = volumetric_fog.step_count;
    let bounding_radius = volumetric_fog.bounding_radius;
    let absorption = volumetric_fog.absorption;
    let scattering = volumetric_fog.scattering;
    let density_factor = volumetric_fog.density_factor;
    let density_texture_offset = volumetric_fog.density_texture_offset;
    let light_tint = volumetric_fog.light_tint;
    let light_intensity = volumetric_fog.light_intensity;
    let jitter_strength = volumetric_fog.jitter_strength;

    // Unpack the view.
    let exposure = view.exposure;

    // Sample the depth to put an upper bound on the length of the ray (as we
    // shouldn't trace through solid objects). If this is multisample, just use
    // sample 0; this is approximate but good enough.
    let frag_coord = position;
    let ndc_end_depth_from_buffer = textureLoad(depth_texture, vec2<i32>(frag_coord.xy), 0);
    let view_end_depth_from_buffer = -position_ndc_to_view(
        frag_coord_to_ndc(vec4(position.xy, ndc_end_depth_from_buffer, 1.0))).z;

    // Calculate the start position of the ray. Since we're only rendering front
    // faces of the AABB, this is the current fragment's depth.
    let view_start_pos = position_ndc_to_view(frag_coord_to_ndc(frag_coord));

    // Calculate the end position of the ray. This requires us to raytrace the
    // three back faces of the AABB to find the one that our ray intersects.
    var end_depth_view = 0.0;
    for (var plane_index = 0; plane_index < 3; plane_index += 1) {
        let plane = volumetric_fog.far_planes[plane_index];
        let other_plane_a = volumetric_fog.far_planes[(plane_index + 1) % 3];
        let other_plane_b = volumetric_fog.far_planes[(plane_index + 2) % 3];

        // Calculate the intersection of the ray and the plane. The ray must
        // intersect in front of us (t > 0).
        let t = -plane.w / dot(plane.xyz, view_start_pos.xyz);
        if (t < 0.0) {
            continue;
        }
        let hit_pos = view_start_pos.xyz * t;

        // The intersection point must be in front of the other backfaces.
        let other_sides = vec2(
            dot(vec4(hit_pos, 1.0), other_plane_a) >= 0.0,
            dot(vec4(hit_pos, 1.0), other_plane_b) >= 0.0
        );

        // If those tests pass, we found our backface.
        if (all(other_sides)) {
            end_depth_view = -hit_pos.z;
            break;
        }
    }

    // Starting at the end depth, which we got above, figure out how long the
    // ray we want to trace is and the length of each increment.
    end_depth_view = min(end_depth_view, view_end_depth_from_buffer);

    // We assume world and view have the same scale here.
    let start_depth_view = -depth_ndc_to_view_z(frag_coord.z);
    let ray_length_view = abs(end_depth_view - start_depth_view);
    let inv_step_count = 1.0 / f32(step_count);
    let step_size_world = ray_length_view * inv_step_count;

    let directional_light_count = lights.n_directional_lights;

    // Calculate the ray origin (`Ro`) and the ray direction (`Rd`) in NDC,
    // view, and world coordinates.
    let Rd_ndc = vec3(frag_coord_to_ndc(position).xy, 1.0);
    let Rd_view = normalize(position_ndc_to_view(Rd_ndc));
    var Ro_world = position_view_to_world(view_start_pos.xyz);
    let Rd_world = normalize(position_ndc_to_world(Rd_ndc) - view.world_position);

    // Offset by jitter.
    let jitter = interleaved_gradient_noise(position.xy, globals.frame_count) * jitter_strength;
    Ro_world += Rd_world * jitter;

    // Use Beer's law [1] [2] to calculate the maximum amount of light that each
    // directional light could contribute, and modulate that value by the light
    // tint and fog color. (The actual value will in turn be modulated by the
    // phase according to the Henyey-Greenstein formula.)
    //
    // [1]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/intro-volume-rendering.html
    //
    // [2]: https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law

    // Use Beer's law again to accumulate the ambient light all along the path.
    var accumulated_color = exp(-ray_length_view * (absorption + scattering)) * ambient_color *
        ambient_intensity;

    // This is the amount of the background that shows through. We're actually
    // going to recompute this over and over again for each directional light,
    // coming up with the same values each time.
    var background_alpha = 1.0;

    // If we have a density texture, transform to its local space.
#ifdef DENSITY_TEXTURE
    let Ro_uvw = (uvw_from_world * vec4(Ro_world, 1.0)).xyz;
    let Rd_step_uvw = mat3x3(uvw_from_world[0].xyz, uvw_from_world[1].xyz, uvw_from_world[2].xyz) *
        (Rd_world * step_size_world);
#endif  // DENSITY_TEXTURE

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

        // Reset `background_alpha` for a new raymarch.
        background_alpha = 1.0;

        // Start raymarching.
        for (var step = 0u; step < step_count; step += 1u) {
            // As an optimization, break if we've gotten too dark.
            if (background_alpha < 0.001) {
                break;
            }

            // Calculate where we are in the ray.
            let P_world = Ro_world + Rd_world * f32(step) * step_size_world;
            let P_view = Rd_view * f32(step) * step_size_world;

            var density = density_factor;
#ifdef DENSITY_TEXTURE
            // Take the density texture into account, if there is one.
            //
            // The uvs should never go outside the (0, 0, 0) to (1, 1, 1) box,
            // but sometimes due to floating point error they can. Handle this
            // case.
            let P_uvw = Ro_uvw + Rd_step_uvw * f32(step);
            if (all(P_uvw >= vec3(0.0)) && all(P_uvw <= vec3(1.0))) {
                density *= textureSampleLevel(density_texture, density_sampler, P_uvw + density_texture_offset, 0.0).r;
            } else {
                density = 0.0;
            }
#endif  // DENSITY_TEXTURE

            // Calculate absorption (amount of light absorbed by the fog) and
            // out-scattering (amount of light the fog scattered away).
            let sample_attenuation = exp(-step_size_world * density * (absorption + scattering));

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
                let light_attenuation = exp(-density * bounding_radius * (absorption + scattering));
                let light_factors_per_step = fog_color * light_tint * light_attenuation *
                    scattering * density * step_size_world * light_intensity * exposure;

                // Modulate the factor we calculated above by the phase, fog color,
                // light color, light tint.
                let light_color_per_step = (*light).color.rgb * phase * light_factors_per_step;

                // Accumulate the light.
                accumulated_color += light_color_per_step * local_light_attenuation *
                    background_alpha;
            }
        }
    }

    // Point lights and Spot lights
    let view_z = view_start_pos.z;
    let is_orthographic = view.clip_from_view[3].w == 1.0;
    let cluster_index = clustering::fragment_cluster_index(frag_coord.xy, view_z, is_orthographic);
    var clusterable_object_index_ranges =
        clustering::unpack_clusterable_object_index_ranges(cluster_index);
    for (var i: u32 = clusterable_object_index_ranges.first_point_light_index_offset;
            i < clusterable_object_index_ranges.first_reflection_probe_index_offset;
            i = i + 1u) {
        let light_id = clustering::get_clusterable_object_id(i);
        let light = &clusterable_objects.data[light_id];
        if (((*light).flags & POINT_LIGHT_FLAGS_VOLUMETRIC_BIT) == 0) {
            continue;
        }

        // Reset `background_alpha` for a new raymarch.
        background_alpha = 1.0;

        // Start raymarching.
        for (var step = 0u; step < step_count; step += 1u) {
            // As an optimization, break if we've gotten too dark.
            if (background_alpha < 0.001) {
                break;
            }

            // Calculate where we are in the ray.
            let P_world = Ro_world + Rd_world * f32(step) * step_size_world;
            let P_view = Rd_view * f32(step) * step_size_world;

            var density = density_factor;

            let light_to_frag = (*light).position_radius.xyz - P_world;
            let V = Rd_world;
            let L = normalize(light_to_frag);
            let distance_square = dot(light_to_frag, light_to_frag);
            let distance_atten = getDistanceAttenuation(distance_square, (*light).color_inverse_square_range.w);
            var local_light_attenuation = distance_atten;
            if (i < clusterable_object_index_ranges.first_spot_light_index_offset) {
                var shadow: f32 = 1.0;
                if (((*light).flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
                    shadow = fetch_point_shadow_without_normal(light_id, vec4(P_world, 1.0));
                }
                local_light_attenuation *= shadow;
            } else {
                // spot light attenuation
                // reconstruct spot dir from x/z and y-direction flag
                var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
                spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
                if ((*light).flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u {
                    spot_dir.y = -spot_dir.y;
                }
                let light_to_frag = (*light).position_radius.xyz - P_world;

                // calculate attenuation based on filament formula https://google.github.io/filament/Filament.html#listing_glslpunctuallight
                // spot_scale and spot_offset have been precomputed
                // note we normalize here to get "l" from the filament listing. spot_dir is already normalized
                let cd = dot(-spot_dir, normalize(light_to_frag));
                let attenuation = saturate(cd * (*light).light_custom_data.z + (*light).light_custom_data.w);
                let spot_attenuation = attenuation * attenuation;

                var shadow: f32 = 1.0;
                if (((*light).flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
                    shadow = fetch_spot_shadow_without_normal(light_id, vec4(P_world, 1.0));
                }
                local_light_attenuation *= spot_attenuation * shadow;
            }
            
            // Calculate absorption (amount of light absorbed by the fog) and
            // out-scattering (amount of light the fog scattered away).
            let sample_attenuation = exp(-step_size_world * density * (absorption + scattering));

            // Process absorption and out-scattering.
            background_alpha *= sample_attenuation;

            let light_attenuation = exp(-density * bounding_radius * (absorption + scattering));
            let light_factors_per_step = fog_color * light_tint * light_attenuation *
                scattering * density * step_size_world * light_intensity * 0.1;

            // Modulate the factor we calculated above by the phase, fog color,
            // light color, light tint.
            let light_color_per_step = (*light).color_inverse_square_range.rgb * light_factors_per_step;

            // Accumulate the light.
            accumulated_color += light_color_per_step * local_light_attenuation *
                background_alpha;
        }
    }

    // We're done! Return the color with alpha so it can be blended onto the
    // render target.
    return vec4(accumulated_color, 1.0 - background_alpha);
}

fn fetch_point_shadow_without_normal(light_id: u32, frag_position: vec4<f32>) -> f32 {
    let light = &clusterable_objects.data[light_id];

    // because the shadow maps align with the axes and the frustum planes are at 45 degrees
    // we can get the worldspace depth by taking the largest absolute axis
    let surface_to_light = (*light).position_radius.xyz - frag_position.xyz;
    let surface_to_light_abs = abs(surface_to_light);
    let distance_to_light = max(surface_to_light_abs.x, max(surface_to_light_abs.y, surface_to_light_abs.z));

    // The normal bias here is already scaled by the texel size at 1 world unit from the light.
    // The texel size increases proportionally with distance from the light so multiplying by
    // distance to light scales the normal bias to the texel size at the fragment distance.
    let depth_offset = (*light).shadow_depth_bias * normalize(surface_to_light.xyz);
    let offset_position = frag_position.xyz + depth_offset;

    // similar largest-absolute-axis trick as above, but now with the offset fragment position
    let frag_ls = offset_position.xyz - (*light).position_radius.xyz ;
    let abs_position_ls = abs(frag_ls);
    let major_axis_magnitude = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));

    // NOTE: These simplifications come from multiplying:
    // projection * vec4(0, 0, -major_axis_magnitude, 1.0)
    // and keeping only the terms that have any impact on the depth.
    // Projection-agnostic approach:
    let zw = -major_axis_magnitude * (*light).light_custom_data.xy + (*light).light_custom_data.zw;
    let depth = zw.x / zw.y;

    // Do the lookup, using HW PCF and comparison. Cubemaps assume a left-handed coordinate space,
    // so we have to flip the z-axis when sampling.
    let flip_z = vec3(1.0, 1.0, -1.0);
    return sample_shadow_cubemap(frag_ls * flip_z, distance_to_light, depth, light_id);
}

fn fetch_spot_shadow_without_normal(light_id: u32, frag_position: vec4<f32>) -> f32 {
    let light = &clusterable_objects.data[light_id];

    let surface_to_light = (*light).position_radius.xyz - frag_position.xyz;

    // construct the light view matrix
    var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
    // reconstruct spot dir from x/z and y-direction flag
    spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
    if (((*light).flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u) {
        spot_dir.y = -spot_dir.y;
    }

    // view matrix z_axis is the reverse of transform.forward()
    let fwd = -spot_dir;
    let offset_position =
        -surface_to_light
        + ((*light).shadow_depth_bias * normalize(surface_to_light));

    // the construction of the up and right vectors needs to precisely mirror the code
    // in render/light.rs:spot_light_view_matrix
    var sign = -1.0;
    if (fwd.z >= 0.0) {
        sign = 1.0;
    }
    let a = -1.0 / (fwd.z + sign);
    let b = fwd.x * fwd.y * a;
    let up_dir = vec3<f32>(1.0 + sign * fwd.x * fwd.x * a, sign * b, -sign * fwd.x);
    let right_dir = vec3<f32>(-b, -sign - fwd.y * fwd.y * a, fwd.y);
    let light_inv_rot = mat3x3<f32>(right_dir, up_dir, fwd);

    // because the matrix is a pure rotation matrix, the inverse is just the transpose, and to calculate
    // the product of the transpose with a vector we can just post-multiply instead of pre-multiplying.
    // this allows us to keep the matrix construction code identical between CPU and GPU.
    let projected_position = offset_position * light_inv_rot;

    // divide xy by perspective matrix "f" and by -projected.z (projected.z is -projection matrix's w)
    // to get ndc coordinates
    let f_div_minus_z = 1.0 / ((*light).spot_light_tan_angle * -projected_position.z);
    let shadow_xy_ndc = projected_position.xy * f_div_minus_z;
    // convert to uv coordinates
    let shadow_uv = shadow_xy_ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

    // 0.1 must match POINT_LIGHT_NEAR_Z
    let depth = 0.1 / -projected_position.z;

    return sample_shadow_map(
        shadow_uv,
        depth,
        i32(light_id) + lights.spot_light_shadowmap_offset,
        SPOT_SHADOW_TEXEL_SIZE
    );
}