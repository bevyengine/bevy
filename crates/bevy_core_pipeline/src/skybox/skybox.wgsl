#import bevy_pbr::mesh_view_types

@group(0) @binding(0)
var skybox: texture_cube<f32>;
@group(0) @binding(1)
var skybox_sampler: sampler;
@group(0) @binding(2)
var<uniform> view: View;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
};

@vertex
fn skybox_vertex(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = view.view_proj * vec4(position, 0.0);
    out.position = position;
    return out;
}

@fragment
fn skybox_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(skybox, skybox_sampler, in.position);
}
