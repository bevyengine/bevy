#define_import_path bevy_gizmos::utils

const EPSILON: f32 = 4.88e-04;

fn calculate_depth(depth_bias: f32, clip: vec4<f32>) -> f32 {
    if depth_bias >= 0. {
        return clip.z * (1. - depth_bias);
    } else {
        // depth * (clip.w / depth)^-depth_bias. So that when -depth_bias is 1.0, this is equal to clip.w
        // and when equal to 0.0, it is exactly equal to depth.
        // the epsilon is here to prevent the depth from exceeding clip.w when -depth_bias = 1.0 
        // clip.w represents the near plane in homogeneous clip space in bevy, having a depth
        // of this value means nothing can be in front of this
        // The reason this uses an exponential function is that it makes it much easier for the 
        // user to chose a value that is convenient for them
        return clip.z * exp2(-depth_bias * log2(clip.w / clip.z - EPSILON));
    }
}
