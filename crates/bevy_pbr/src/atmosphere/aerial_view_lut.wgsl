#import bevy_pbr::{
    mesh_view_types::{Lights, DirectionalLight},
    atmosphere::types::{Atmosphere, AtmosphereSettings},
}

#import bevy_render::view::View;

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(2) var<uniform> lights: Lights;
@group(0) @binding(3) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(4) var tranmittance_lut_sampler: sampler;
@group(0) @binding(5) var multiscattering_lut: texture_2d<f32>;
@group(0) @binding(6) var multiscattering_lut_sampler: sampler;
@group(0) @binding(7) var aerial_view_lut: texture_storage_3d<rgba16float, write>;

@compute
@workgroup_size(16, 16, 1) //TODO: this approach makes it so closer slices get fewer samples. But we also expect those to have less scattering. So win/win?
fn main(@builtin(global_invocation_id) idx: vec2<u32>) {
    if any(idx > settings.aerial_view_lut_size.xy) { return; }
    let optical_depth: f32 = 0.0;

    let view_dir = vec3(0.0); //TODO: bind view and do actual transform
    let in_scattering = vec3(0.0);
    for (let z = settings.aerial_view_lut_size.z - 1; z >= 0; z--) { //reversed loop to make coords match reversed Z
        let clip_pos = vec3(vec2<f32>(idx) + 0.5, f32(z) + 0.5) / vec3<f32>(settings.aerial_view_lut_size);
        //TODO: sample transmittance some n times per z step 
        let transmittance_to_sample = exp(-optical_depth);

        //TODO: since matrices are linear we could do some fancy things to avoid a matrix mult, and only add a constant each loop (+ depth weirdness maybe)
        let view_pos = vec3(0.0); //TODO: bind view and do actual transform

        let dt = 0.0; //TODO: dt in the equation 1 integral. don't forget, otherwise our units are fully wrong;

        for (let i = 0u; i < lights.n_directional_lights; i++) {
            let light = lights.directional_lights[i];
            //TODO: get reflected light from surface
            let transmittance_to_light = vec3(0.0) //TODO: get from transmittance LUT
            let rayleigh_phase = 0.0; //TODO: calculate from sun_dir, view_dir.
            let mie_phase = 0.0; //TODO: calculate from sun_dir, view_dir. Use henyey-greenstein from pcwalton's stuff
            let vis = true; //TODO: check for planet intersection in sun
            let psi_ms = vec3(0.0) //multiscattering thingy. needs better name. TODO: get from LUT
            //TODO: multiply all stuff, add to in_scattering
        }

        textureStore(aerial_view_lut, vec3(idx.xy, z), vec4(in_scattering));
    }
}
