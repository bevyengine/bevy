#define_import_path bevy_pbr::gtao_utils

#import bevy_pbr::utils PI, HALF_PI

// Approximates single-bounce ambient occlusion to multi-bounce ambient occlusion
// https://blog.selfshadow.com/publications/s2016-shading-course/activision/s2016_pbs_activision_occlusion.pdf#page=78
fn gtao_multibounce(visibility: f32, base_color: vec3<f32>) -> vec3<f32> {
    let a = 2.0404 * base_color - 0.3324;
    let b = -4.7951 * base_color + 0.6417;
    let c = 2.7552 * base_color + 0.6903;
    let x = vec3<f32>(visibility);
    return max(x, ((x * a + b) * x + c) * x);
}

fn fast_sqrt(x: f32) -> f32 {
    return bitcast<f32>(0x1fbd1df5 + (bitcast<i32>(x) >> 1u));
}

fn fast_acos(in_x: f32) -> f32 {
    let x = abs(in_x);
    var res = -0.156583 * x + HALF_PI;
    res *= fast_sqrt(1.0 - x);
    return select(PI - res, res, in_x >= 0.0);
}
