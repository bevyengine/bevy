//! Example of a fragment shader for a custom material setup to correctly handle
//! order independent transparency.
//! See examples/3d/order_independent_transpatency.rs

// As an hard-coded always transparent material it is only meant to be forward rendered
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::utils

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif // OIT_ENABLED

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material_color: vec4<f32>;

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {

    // This produces a noise-like varying transparency.
    // Using world coordinates for simplicity, but local mesh coordinates
    // would be required if the mesh moved.
    let alpha = gradient_noise(in.world_position.xyz, 5.0);
    let color = vec4f(material_color.rgb, alpha);

#ifdef OIT_ENABLED
    oit_draw(in.position, color);
    discard;
#endif // OIT_ENABLED

    return color;
}

fn gradient_noise(coord: vec3f, scale: f32) -> f32 {
    let grid_coord = vec3u(scale * abs(coord));
    let f = fract(scale * abs(coord)); 

    let g0 = rand3(grid_coord + vec3u(0, 0, 0));
    let g1 = rand3(grid_coord + vec3u(1, 0, 0));
    let g2 = rand3(grid_coord + vec3u(0, 1, 0));
    let g3 = rand3(grid_coord + vec3u(1, 1, 0));
    let g4 = rand3(grid_coord + vec3u(0, 0, 1));
    let g5 = rand3(grid_coord + vec3u(1, 0, 1));
    let g6 = rand3(grid_coord + vec3u(0, 1, 1));
    let g7 = rand3(grid_coord + vec3u(1, 1, 1));

    let noise = mix(
                    mix(
                        mix(dot(g0, f - vec3f(0.0, 0.0, 0.0)),
                            dot(g1, f - vec3f(1.0, 0.0, 0.0)), f.x),
                        mix(dot(g2, f - vec3f(0.0, 1.0, 0.0)),
                            dot(g3, f - vec3f(1.0, 1.0, 0.0)), f.x),
                        f.y),
                    mix(
                        mix(dot(g4, f - vec3f(0.0, 0.0, 1.0)),
                            dot(g5, f - vec3f(1.0, 0.0, 1.0)), f.x),
                        mix(dot(g6, f - vec3f(0.0, 1.0, 1.0)),
                            dot(g7, f - vec3f(1.0, 1.0, 1.0)), f.x),
                        f.y),
                    f.z);

    return noise;
}

fn rand3(seed: vec3u) -> vec3f {
    var rng: u32 = (seed.x * 3966231743u) ^ (seed.y * 3928936651u) ^ (seed.z * 53134757u);
    return rand_vec3f(&rng);
}

fn rand_vec3f(state: ptr<function, u32>) -> vec3<f32> {
    return vec3f(utils::rand_f(state), utils::rand_f(state), utils::rand_f(state));
}
