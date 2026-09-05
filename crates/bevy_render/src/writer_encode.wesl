//! Encodes a fragment's output color for the view's compositing space, so
//! blending happens in that space. The upscaling blit decodes back to linear.

@if(COMPOSITING_SPACE_SRGB)
import package::color_operations::linear_to_srgb;
@if(COMPOSITING_SPACE_OKLAB)
import package::color_operations::linear_rgb_to_oklab;

fn writer_encode(color_in: vec4<f32>) -> vec4<f32> {
    var color = color_in;
@if(COMPOSITING_SPACE_SRGB) {
    color = vec4(linear_to_srgb(color.rgb), color.a);
}
@if(COMPOSITING_SPACE_OKLAB) {
    color = vec4(linear_rgb_to_oklab(color.rgb), color.a);
}
    return color;
}
