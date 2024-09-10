#import bevy_pbr::atmosphere::types::Atmosphere;

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> settings: AtmosphereSettings;
@group(0) @binding(2) var<uniform> lights: Lights;
@group(0) @binding(3) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(4) var multiscattering_lut: texture_2d<f32>;

@fragment
fn main() -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 0.0, 0.0);
}
