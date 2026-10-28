import bevy_pbr::atmosphere::types::Atmosphere;

// Bound in the per-view group of every pipeline that shades lights.
// Views without an atmosphere bind fallbacks with a zero `outer_radius`.
@group(1) @binding(10) var atmosphere_transmittance_lut: texture_2d<f32>;
@group(1) @binding(11) var atmosphere_transmittance_sampler: sampler;
@group(1) @binding(12) var<uniform> atmosphere: Atmosphere;
