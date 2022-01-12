var<private> verts : array<vec4<f32>, 6> = array<vec4<f32>, 6>(
  vec4<f32>(-1.,  1., 0., 1.),
  vec4<f32>(-1., -1., 0., 1.),
  vec4<f32>( 1.,  1., 0., 1.),

  vec4<f32>( 1.,  1., 0., 1.),
  vec4<f32>( 1., -1., 0., 1.),
  vec4<f32>(-1., -1., 0., 1.),
);

struct BackgroundMaterial {
  color: vec4<f32>;
  time: f32;
  resolution: vec2<f32>;
};

[[group(0), binding(0)]]
var<uniform> u: BackgroundMaterial;

[[stage(vertex)]]
fn vert([[builtin(vertex_index)]] v_index: u32) -> [[builtin(position)]] vec4<f32> {
  return verts[v_index];
}

[[stage(fragment)]]
fn frag([[builtin(position)]] frag_coord: vec4<f32>) -> [[location(0)]] vec4<f32> {
  //let c = vec3<f32>(0.1, 0.1,0.6);
  let c = smoothStep(-2., 2., sin(u.time + (frag_coord.x + frag_coord.y) / 100.)) * u.color.rgb;
  return vec4<f32>(c, u.color.a);
}