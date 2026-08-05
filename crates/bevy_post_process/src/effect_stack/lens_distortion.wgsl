// The lens distortion postprocessing effect.

#define_import_path bevy_post_process::effect_stack::lens_distortion

// See `bevy_post_process::effect_stack::LensDistortion` for more
// information on these fields.
struct LensDistortionSettings {
    intensity: f32,
    scale: f32,
    multiplier: vec2<f32>,
    center: vec2<f32>,
    edge_curvature: f32,
    unused: u32,
}

const TAU: f32 = radians(360);
const ONE: vec2<f32> = vec2<f32>(1.0, 1.0);

// The settings supplied by the developer.
@group(0) @binding(5) var<uniform> lens_distortion_settings: LensDistortionSettings;

fn lens_distortion(uv: vec2<f32>) -> vec2<f32> {
    let intensity = lens_distortion_settings.intensity;
    let multiplier = lens_distortion_settings.multiplier;
    let center = lens_distortion_settings.center;
    let uv_centered = uv - center;

    // This shader takes the UV as a vector centered on the screen and splits it into two parts: the direction, and the magnitude.
    // The direction is used to add warp the corners more than the sides.
    // The magnitude is the value that is actually distorted.
    // The adjusted and distorted direction and magnitudes are then recombined and scaled to map the original UV coordinates to the new output coordinates.

    // This will scale the influence of the distortion, but not the image.
    // A multiplier of {1.0, 1.0} will place the most distorted parts of the output exactly on the corners.
    // A multiplier of {1.0, 2.0} would place the top and bottom points of highest influence 0.5 screens above and below the rendered output respectively.
    // This can be used to always guarantee an even distortion using the ratio between the screen width and height.
    let uv_multiplied = uv_centered * multiplier;
    
    let magnitude = length(uv_multiplied);
    let m2 = magnitude * magnitude;
    let direction = normalize(uv_multiplied);

    // Correct for the uv multiplier to prevent scaling the image along with the distortion.
    let direction_adjusted = direction / multiplier;

    // Gets a value from -TAU to TAU for how aligned the direction is with the closest corner.  0 is perpendicular.
    let influence = dot(abs(direction), ONE);

    // Adjusts the output so that the perpendicular angle (where the output is zero) is always 45 degrees from the corner (aligned with the axes)
    let adjust = ((influence - TAU) * 0.5) + (TAU * 2);

    // Maintains the correlation between k2 and k1, while ensuring the sign of k2 is determined solely by `edge_curvature` rather than being influenced by intensity.
    // Based on maths that can be found here: https://en.wikipedia.org/wiki/Distortion_(optics)#Software_correction
    let k1 = intensity * adjust;
    let k2 = k1 * intensity * lens_distortion_settings.edge_curvature;

    // Calculating the distortion based on distance from center and the two variables k1 and k2 to create a compound parabola.
    let magnitude_distorted = magnitude * (1.0 + (k1 + k2 * m2) * m2);

    // Re-combining the direction and magnitude after.
    let uv_distorted = direction_adjusted * magnitude_distorted + center;

    // Compensates for the distortion pushing pixels outside the [0,1] UV bounds.
    let uv_scaled = ((uv_distorted - center) / lens_distortion_settings.scale + center);

    return uv_scaled;
}
