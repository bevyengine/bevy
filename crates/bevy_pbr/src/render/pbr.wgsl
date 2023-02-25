#import bevy_pbr::pbr_fragment

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return pbr_fragment(in);
}
