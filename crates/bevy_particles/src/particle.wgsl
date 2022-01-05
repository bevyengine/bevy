// TODO: try merging this block with the binding?
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct PositionBuffer { data: array<vec4<f32>>; };
struct SizeBuffer { data: array<f32>; };
struct ColorBuffer { data: array<vec4<f32>>; };

[[group(1), binding(0)]]
var<storage, read> positions: PositionBuffer;
[[group(1), binding(1)]]
var<storage, read> sizes: SizeBuffer;
[[group(1), binding(2)]]
var<storage, read> colors:ColorBuffer;
[[group(2), binding(0)]]
var base_color_texture: texture_2d<f32>;
[[group(2), binding(1)]]
var base_color_sampler: sampler;

struct VertexInput {
  [[builtin(vertex_index)]] vertex_idx: u32;
};

struct VertexOutput {
  [[builtin(position)]] position: vec4<f32>;
  [[location(0)]] color: vec4<f32>;
  [[location(1)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main(model: VertexInput) -> VertexOutput {
  let vert_idx = model.vertex_idx % 6u;
  let particle_idx = model.vertex_idx / 6u;

  // Compute the vertices and UVs of a quad.
  let tri_vert_idx = vert_idx % 3u;
  let direction = f32(vert_idx / 3u) - 0.5;
  let vertex_position = 
    direction *
    vec2<f32>(
      sign(f32(tri_vert_idx & 1u) - 0.5), 
      sign(f32((tri_vert_idx >> 1u) & 1u) - 0.5)
    );
  let uv = vec2<f32>(vertex_position.x + 0.5, -vertex_position.y + 0.5);

  // Uses the view projection matrix to compute the world-space movement directions
  // TODO: This is actually constant for all billboards and is best done CPU-side.
  let camera_right = 
    normalize(vec3<f32>(view.view_proj.x.x, view.view_proj.y.x, view.view_proj.z.x));
  let camera_up = 
    normalize(vec3<f32>(view.view_proj.x.y, view.view_proj.y.y, view.view_proj.z.y));

  let particle_position = positions.data[particle_idx].xyz;
  let theta = positions.data[particle_idx].w;
  let size = sizes.data[particle_idx];
  let sin_cos = vec2<f32>(cos(theta), sin(theta));
  
  let rotation = mat2x2<f32>(
    vec2<f32>(sin_cos.x, -sin_cos.y),
    vec2<f32>(sin_cos.y, sin_cos.x),
  );

  let vertex_position = rotation * vertex_position;

  var world_space: vec3<f32> = 
    particle_position + 
    (camera_right * vertex_position.x * size) + 
    (camera_up * vertex_position.y * size);

  var out: VertexOutput;
  out.position = view.view_proj * vec4<f32>(world_space, 1.0);
  out.color = colors.data[particle_idx];
  out.uv = uv;
  return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  var output_color: vec4<f32> = in.color;
#ifdef BASE_COLOR_TEXTURE
  output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
#endif
  return output_color;
}
