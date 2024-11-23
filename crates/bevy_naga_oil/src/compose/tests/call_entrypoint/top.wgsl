#import include as Inc

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>,
) -> @location(0) vec4<f32>  {
    return Inc::fragment(frag_coord);
}