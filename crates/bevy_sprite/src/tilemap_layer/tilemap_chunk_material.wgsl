#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_view_bindings::view,
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(5) tile_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tile_index: u32,
}

@group(2) @binding(0) var tileset: texture_2d_array<f32>;
@group(2) @binding(1) var tileset_sampler: sampler;
@group(2) @binding(2) var tile_data: texture_2d<u32>;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0)
    );

    out.position = mesh_functions::mesh2d_position_world_to_clip(world_position);
    out.uv = vertex.uv;
    out.tile_index = vertex.tile_index;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let chunk_size = textureDimensions(tile_data, 0);
    let tile_xy = vec2<u32>(
        in.tile_index % chunk_size.x,
        in.tile_index / chunk_size.x
    );

    let tile = textureLoad(tile_data, tile_xy, 0);
    let tile_id = tile.r;
    let visible = tile.g != 0u;

    if (tile_id == 0xffffu || !visible) {
        discard;
    }

    let color_r = f32(tile.b & 0xFFu) / 255.0;
    let color_g = f32((tile.b >> 8u) & 0xFFu) / 255.0;
    let color_b = f32(tile.a & 0xFFu) / 255.0;
    let color_a = f32((tile.a >> 8u) & 0xFFu) / 255.0;

    let tile_color = vec4<f32>(color_r, color_g, color_b, color_a);
    let tex_color = textureSample(tileset, tileset_sampler, in.uv, tile_id);
    let final_color = tex_color * tile_color;

    // Alpha-based visibility - discard if fully transparent
    if (final_color.a < 0.001) {
        discard;
    }

    return final_color;
}