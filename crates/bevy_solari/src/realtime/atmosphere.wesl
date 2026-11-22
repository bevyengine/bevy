import bevy_pbr::atmosphere::functions::directional_light_transmittance;
import package::scene::sampling::{calculate_resolved_light_contribution, LightContribution, ResolvedLightSample};
import package::realtime::bindings::{atmosphere, atmosphere_transmittance_lut, atmosphere_transmittance_sampler};

enable wgpu_ray_query;

fn calculate_light_contribution(resolved_light_sample: ResolvedLightSample, ray_origin: vec3<f32>, origin_world_normal: vec3<f32>) -> LightContribution {
    var light_contribution = calculate_resolved_light_contribution(resolved_light_sample, ray_origin, origin_world_normal);
    // Skip views without an atmosphere and lights behind the surface.
    if atmosphere.outer_radius > 0.0 && resolved_light_sample.world_position.w == 0.0 && dot(light_contribution.wi, origin_world_normal) > 0.0 {
        light_contribution.radiance *= directional_light_transmittance(
            atmosphere,
            atmosphere_transmittance_lut,
            atmosphere_transmittance_sampler,
            ray_origin,
            light_contribution.wi,
            // Light samples are points on the sun's disk, so they average to its visible fraction.
            0.0,
        );
    }
    return light_contribution;
}
