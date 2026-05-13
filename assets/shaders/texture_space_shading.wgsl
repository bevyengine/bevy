#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct TextureSpaceShadingSettings {
    t: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(0) var<uniform> settings: TextureSpaceShadingSettings;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
  var pos = vec3(0.);
  var color = vec3(0.);
  // Invert mapping from UV back to local cube space.
  // Also select a face color based on the cube face.
  if in.uv.y < 0.333333 {
      if in.uv.x < 0.333333 {
          // front
          pos = vec3(in.uv.x * 6 - 1, in.uv.y * 6 - 1, 1);
          color = vec3(0, 0, 1);
        } else if in.uv.x < 0.666666 {
          // back
          pos = vec3(3 - in.uv.x * 6, in.uv.y * 6 - 1, -1);
          color = vec3(1, 1, 0);
        } else {
          // right
          pos = vec3(1, in.uv.y * 6 - 1, 5 - in.uv.x * 6);
          color = vec3(1, 0, 0);
      }
    } else {
      if in.uv.x < 0.333333 {
          // left
          pos = vec3(-1, in.uv.y * 6 - 3, in.uv.x * 6 - 1);
          color = vec3(0, 1, 1);
        } else if in.uv.x < 0.666666 {
          // top
          pos = vec3(in.uv.x * 6 - 3, 1, 3 - in.uv.y * 6);
          color = vec3(0, 1, 0);
        } else {
          // bottom
          pos = vec3(in.uv.x * 6 - 5, -1, in.uv.y * 6 - 3);
          color = vec3(1, 0, 1);
      }
  }
  let g = fract(pos * 2); // Convert [-1, 1] to a 4x4 grid
  // Compute distance from center of each cell, then wrap that around based on the settings
  let d = fract(length(g - 0.5) * (3 + 7 * settings.t));
  // Invert the color based on d
  if d > 0.5 {
    return vec4(1 - color, 1);
  } else {
    return vec4(color, 1);
  }
}
