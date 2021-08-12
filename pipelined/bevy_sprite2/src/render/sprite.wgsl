// TODO: try merging this block with the binding?
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var view: View;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

struct SpriteTransformCols {
    [[location(0)]] col0: vec4<f32>;
    [[location(1)]] col1: vec4<f32>;
    [[location(2)]] col2: vec4<f32>;
    [[location(3)]] col3: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    sprite_transform_cols: SpriteTransformCols,

    [[location(4)]] sprite_size: vec2<f32>,

    [[location(5)]] uv_min: vec2<f32>,
    [[location(6)]] uv_size: vec2<f32>,

    [[builtin(vertex_index)]] vertex_id: u32
) -> VertexOutput {
    let sprite_transform = mat4x4<f32>(
        sprite_transform_cols.col0,
        sprite_transform_cols.col1,
        sprite_transform_cols.col2,
        sprite_transform_cols.col3,
    );

    var quad_verts: mat4x4<f32> = mat4x4<f32>(
        vec4<f32>(0.5, -0.5, 0., 1.),   // bottom right
        vec4<f32>(-0.5, -0.5, 0., 1.),  // bottom left
        vec4<f32>(0.5, 0.5, 0., 1.),    // top right
        vec4<f32>(-0.5, 0.5, 0., 1.),   // top left
    );
    var quad_uvs: mat4x2<f32> = mat4x2<f32>(
        vec2<f32>(1.0, 1.0),    // bottom right
        vec2<f32>(0.0, 1.0),    // bottom left
        vec2<f32>(1.0, 0.0),    // top right
        vec2<f32>(0.0, 0.0),    // top left    
    );
    let vert_pos = quad_verts[vertex_id] * vec4<f32>(sprite_size, 1.0, 1.0);
    let vert_uv = uv_min + uv_size * quad_uvs[vertex_id];

    var out: VertexOutput;
    out.uv = vert_uv;
    out.position = view.view_proj * sprite_transform * vert_pos;
    return out;
} 

[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var sprite_sampler: sampler;

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, in.uv);
}