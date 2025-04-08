#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput
}

struct TilemapChunkMaterial {
    chunk_size: vec2<u32>,
    tile_size: vec2<u32>,
};

@group(2) @binding(0) var<uniform> material: TilemapChunkMaterial;
@group(2) @binding(1) var tileset: texture_2d_array<f32>;
@group(2) @binding(2) var tileset_sampler: sampler;
@group(2) @binding(3) var<storage> indices: array<u32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let tile_uv = mesh.uv * vec2<f32>(material.chunk_size);
    let tile_pos = clamp(vec2<u32>(floor(tile_uv)), vec2<u32>(0), material.chunk_size - 1);
    let index = tile_pos.y * material.chunk_size.x + tile_pos.x;
    let tile_index = indices[index];

    if tile_index == 0xffffffffu {
        discard;
    }

    let local_uv = fract(tile_uv);
    let color = textureSample(tileset, tileset_sampler, local_uv, tile_index);
    if (color.a < 0.001) {
        discard;
    }
    return color;
}