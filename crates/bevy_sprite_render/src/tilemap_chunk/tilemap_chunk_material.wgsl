#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_view_bindings::view,
    mesh2d_vertex_output::VertexOutput,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var tileset: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tileset_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var tile_data: texture_2d<u32>;

// NegYPosX = default
// PosYPosX = 1
// NegYNegX = 2
// PosYNegX = 3
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> tiling_direction: u32;

struct TileData {
    tileset_index: u32,
    color: vec4<f32>,
    visible: bool,
}

fn get_tile_data(coord: vec2<u32>) -> TileData {
    let data = textureLoad(tile_data, coord, 0);

    let tileset_index = data.r;

    let color_r = f32(data.g & 0xFFu) / 255.0;
    let color_g = f32((data.g >> 8u) & 0xFFu) / 255.0;
    let color_b = f32(data.b & 0xFFu) / 255.0;
    let color_a = f32((data.b >> 8u) & 0xFFu) / 255.0;

    let color = vec4<f32>(color_r, color_g, color_b, color_a);

    let visible = data.a != 0u;

    return TileData(tileset_index, color, visible);
}

fn get_tile_coord(input_coord: vec2<f32>, chunk_size: vec2<u32>, tiling_direction: u32) -> vec2<u32> {
    var coord = clamp(vec2<u32>(floor(input_coord)), vec2<u32>(0), chunk_size - 1);
    if (tiling_direction == 0) {
        return coord;
    }
    if (tiling_direction == 1) {
        coord.y = chunk_size.y - 1 - coord.y;
        return coord;
    }
    if (tiling_direction == 2) {
        coord.x = chunk_size.x - 1 - coord.x;
        return coord;
    }
    if (tiling_direction == 3) {
        coord.y = chunk_size.y - 1 - coord.y;
        coord.x = chunk_size.x - 1 - coord.x;
        return coord;
    }
    return coord;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let chunk_size = textureDimensions(tile_data, 0);
    let tile_uv = in.uv * vec2<f32>(chunk_size);
    let tile_coord = get_tile_coord(tile_uv, chunk_size, tiling_direction);

    let tile = get_tile_data(tile_coord);

    if (tile.tileset_index == 0xffffu || !tile.visible) {
        discard;
    }

    let local_uv = fract(tile_uv);
    let tex_color = textureSample(tileset, tileset_sampler, local_uv, tile.tileset_index);
    let final_color = tex_color * tile.color;

    if (final_color.a < 0.001) {
        discard;
    }

    return final_color;
}