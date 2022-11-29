#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::utils

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var our_sampler: sampler;

@group(1) @binding(2)
var texture_storage: texture_storage_2d<rgba8unorm, read_write>;

@fragment
fn fragment(
    @builtin(position) position: vec4<f32>,
    #import bevy_sprite::mesh2d_vertex_output
) -> @location(0) vec4<f32> {
    // Get screen position with coordinates from 0 to 1
    let uv = coords_to_viewport_uv(position.xy, view.viewport);

    let xy = vec2<i32>(i32(position.x), i32(position.y));

    var input_color = textureSample(texture, our_sampler, uv);

    var pre_output_color = input_color + textureLoad(texture_storage, vec2<i32>(i32(position.x), i32(position.y)));
    let output_color = vec4<f32>(clamp(pre_output_color.r, 0.0, 1.0), clamp(pre_output_color.g, 0.0, 1.0), clamp(pre_output_color.b, 0.0, 1.0), 1.0);

    textureStore(texture_storage, xy, pre_output_color * 0.8);

    return output_color;
}
