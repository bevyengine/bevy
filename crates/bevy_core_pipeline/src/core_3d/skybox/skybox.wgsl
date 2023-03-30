#import bevy_pbr::mesh_view_types

@group(0) @binding(0)
var skybox: texture_cube<f32>;
@group(0) @binding(1)
var skybox_sampler: sampler;
@group(0) @binding(2)
var<uniform> view: View;

@vertex
fn skybox_vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    var out = view.projection * vec4(position, 1.0);
    out.z = 0.0;
    return out;
}

@fragment
fn skybox_fragment(@builtin(position) clip_position: vec4<f32>) -> @location(0) vec4<f32> {
    return textureSample(skybox, skybox_sampler, clip_position.xyz);
}
