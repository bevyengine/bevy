#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Pipeline-overridable constant: number of discrete color steps.
// This value is substituted at pipeline compilation time by Bevy's pipeline cache
// when the material's `specialize` method pushes it into `descriptor.constants`.
override LEVELS: f32 = 4.0;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Quantize UV.x to LEVELS discrete steps, then derive color from it.
    // This ensures exactly LEVELS distinct colors appear across the gradient.
    let t = floor(in.uv.x * LEVELS) / LEVELS;
    return vec4(t, t * 0.4, 1.0 - t, 1.0);
}
