#import bevy_pbr::mesh_view_bindings

struct Percentages {
    red: vec3<f32>,
    green: vec3<f32>,
    blue: vec3<f32>,
};

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var our_sampler: sampler;

@group(1) @binding(2)
var<uniform> p: Percentages;

@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    #import bevy_sprite::mesh2d_vertex_output
) -> @location(0) vec4<f32> {
    // Get screen position with coordinates from 0 to 1
    let uv = position.xy / vec2<f32>(view.width, view.height);

    var c = textureSample(texture, our_sampler, uv);

    return vec4<f32>(
        c.r * p.red.x + c.g * p.red.y + c.b * p.red.z,
        c.r * p.green.x + c.g * p.green.y + c.b * p.green.z,
        c.r * p.blue.x + c.g * p.blue.y + c.b * p.blue.z,
        c.a
    );
}
