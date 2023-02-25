#import bevy_pbr::pbr_fragment

@group(1) @binding(100)
var<uniform> quantize_steps: u32;

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // call to the standard pbr fragment shader
    var output_color = pbr_fragment(in);

    // we can then modify the results using the extended material data
    output_color = vec4<f32>(vec4<u32>(output_color * f32(quantize_steps))) / f32(quantize_steps);
    return output_color;
}
