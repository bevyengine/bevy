//! Performs depth of field postprocessing, with both Gaussian and bokeh kernels.
//!
//! Gaussian blur is performed as a separable convolution: first blurring in the
//! X direction, and then in the Y direction. This is asymptotically more
//! efficient than performing a 2D convolution.
//!
//! The Bokeh blur uses a similar, but more complex, separable convolution
//! technique. The algorithm is described in Colin Barré-Brisebois, "Hexagonal
//! Bokeh Blur Revisited" [1]. It's motivated by the observation that we can use
//! separable convolutions not only to produce boxes but to produce
//! parallelograms. Thus, by performing three separable convolutions in sequence,
//! we can produce a hexagonal shape. The first and second convolutions are done
//! simultaneously using multiple render targets to cut the total number of
//! passes down to two.
//!
//! [1]: https://colinbarrebrisebois.com/2017/04/18/hexagonal-bokeh-blur-revisited-part-2-improved-2-pass-version/

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;
import bevy_pbr::render::view_transformations::depth_ndc_to_view_z;
import package::gaussian_blur::gaussian_blur;
import bevy_render::view::View;

/// Parameters that control the depth of field effect. See
/// `bevy_post_process::dof::DepthOfFieldUniform` for information on what these
/// parameters mean.
struct DepthOfFieldParams {
    /// The distance in meters to the location in focus.
    focal_distance: f32,

    /// The premultiplied factor that we scale the circle of confusion by.
    ///
    /// This is calculated as:
    /// ```text
    /// sensor_height / (aperture_f_stops * view_height * (view_height - sensor_height))
    /// ```
    /// `view_height` is the height of the view at the focal plane. See
    /// `calculate_circle_of_confusion` below for the derivation.
    coc_scale_factor: f32,

    /// The maximum diameter, in pixels, that we allow a circle of confusion to be.
    ///
    /// A circle of confusion essentially describes the size of a blur.
    ///
    /// This value is nonphysical but is useful for avoiding pathologically-slow
    /// behavior.
    max_circle_of_confusion_diameter: f32,

    /// The depth value that we clamp distant objects to. See the comment in
    /// [`DepthOfField`] for more information.
    max_depth: f32,
}

/// The first bokeh pass outputs to two render targets. We declare them here.
struct DualOutput {
    /// The vertical output.
    @location(0) output_0: vec4<f32>,
    /// The diagonal output.
    @location(1) output_1: vec4<f32>,
}

// @group(0) @binding(0) is `mesh_view_bindings::view`.

/// The depth texture for the main view.
@if(MULTISAMPLED)
@group(0) @binding(1) var depth_texture: texture_depth_multisampled_2d;
@else   // MULTISAMPLED
@group(0) @binding(1) var depth_texture: texture_depth_2d;

/// The main color texture.
@group(0) @binding(2) var color_texture_a: texture_2d<f32>;

/// The auxiliary color texture that we're sampling from. This is only used as
/// part of the second bokeh pass.
@if(DUAL_INPUT)
@group(0) @binding(3) var color_texture_b: texture_2d<f32>;

// The global uniforms, representing data backed by buffers shared among all
// views in the scene.

/// The parameters that control the depth of field effect.
@group(1) @binding(0) var<uniform> dof_params: DepthOfFieldParams;

/// The sampler that's used to fetch texels from the source color buffer.
@group(1) @binding(1) var color_texture_sampler: sampler;

/// cos(-30°), used for the bokeh blur.
const COS_NEG_FRAC_PI_6: f32 = 0.8660254037844387;
/// sin(-30°), used for the bokeh blur.
const SIN_NEG_FRAC_PI_6: f32 = -0.5;
/// cos(-150°), used for the bokeh blur.
const COS_NEG_FRAC_PI_5_6: f32 = -0.8660254037844387;
/// sin(-150°), used for the bokeh blur.
const SIN_NEG_FRAC_PI_5_6: f32 = -0.5;

/// Calculates and returns the diameter (not the radius) of the [circle of
/// confusion].
///
/// [circle of confusion]: https://en.wikipedia.org/wiki/Circle_of_confusion
fn calculate_circle_of_confusion(in_frag_coord: vec4<f32>) -> f32 {
    // Unpack the depth of field parameters.
    let focus = dof_params.focal_distance;
    let scale = dof_params.coc_scale_factor;
    let max_coc_diameter = dof_params.max_circle_of_confusion_diameter;

    // Sample the depth.
    let frag_coord = vec2<i32>(floor(in_frag_coord.xy));
    let raw_depth = textureLoad(depth_texture, frag_coord, 0);
    let depth = min(-depth_ndc_to_view_z(raw_depth), dof_params.max_depth);

    // Calculate the circle of confusion, as a fraction of the view height.
    //
    // Let h be the sensor height, N the f-stops, s the focal distance, d the
    // depth, δ = d - s, and V the height of the view at the focal plane. The
    // perspective formula from Wikipedia [1] is:
    //
    //     coc = f² |δ| / (h N d (s - f)),  where f = h / (2 tan(fov / 2))
    //
    // V = 2 s tan(fov / 2) = s h / f, so f = s h / V and s - f = s (V - h) / V.
    // Substituting:
    //
    //     coc = h / (N V (V - h)) · |δ| · s / d
    //
    // The first factor is `scale`.
    //
    // An orthographic camera is the limit of a perspective camera whose FOV
    // goes to zero while it pulls back (s → ∞), with V and the focal plane
    // fixed in the world. δ is then unchanged, d = s + δ, and s / d → 1, so the
    // orthographic formula drops that factor. With the same `scale`, switching
    // between projections doesn't change the blur.
    //
    // [1]: https://en.wikipedia.org/wiki/Circle_of_confusion#Determining_a_circle_of_confusion_diameter_from_the_object_field
    var candidate_coc = scale * abs(depth - focus);
    if (view.clip_from_view[3][3] != 1.0) {
        candidate_coc *= focus / depth;
    }

    let framebuffer_size = vec2<f32>(textureDimensions(color_texture_a));
    return clamp(candidate_coc * framebuffer_size.y, 0.0, max_coc_diameter);
}

/// Performs a box blur in a single direction, sampling `color_texture_a`.
///
/// * `frag_coord` is the screen-space pixel coordinate of the fragment (i.e. the
///   `position` input to the fragment).
///
/// * `coc` is the diameter (not the radius) of the circle of confusion for this
///   fragment.
///
/// * `frag_offset` is the vector, in screen-space units, from one sample to the
///   next. This need not be horizontal or vertical.
fn box_blur_a(frag_coord: vec4<f32>, coc: f32, frag_offset: vec2<f32>) -> vec4<f32> {
    let support = i32(round(coc * 0.5));
    let uv = frag_coord.xy / vec2<f32>(textureDimensions(color_texture_a));
    let offset = frag_offset / vec2<f32>(textureDimensions(color_texture_a));

    // Accumulate samples in a single direction.
    var sum = vec3(0.0);
    for (var i = 0; i <= support; i += 1) {
        sum += textureSampleLevel(
            color_texture_a, color_texture_sampler, uv + offset * f32(i), 0.0).rgb;
    }

    return vec4(sum / vec3(1.0 + f32(support)), 1.0);
}

/// Performs a box blur in a single direction, sampling `color_texture_b`.
///
/// * `frag_coord` is the screen-space pixel coordinate of the fragment (i.e. the
///   `position` input to the fragment).
///
/// * `coc` is the diameter (not the radius) of the circle of confusion for this
///   fragment.
///
/// * `frag_offset` is the vector, in screen-space units, from one sample to the
///   next. This need not be horizontal or vertical.
@if(DUAL_INPUT)
fn box_blur_b(frag_coord: vec4<f32>, coc: f32, frag_offset: vec2<f32>) -> vec4<f32> {
    let support = i32(round(coc * 0.5));
    let uv = frag_coord.xy / vec2<f32>(textureDimensions(color_texture_b));
    let offset = frag_offset / vec2<f32>(textureDimensions(color_texture_b));

    // Accumulate samples in a single direction.
    var sum = vec3(0.0);
    for (var i = 0; i <= support; i += 1) {
        sum += textureSampleLevel(
            color_texture_b, color_texture_sampler, uv + offset * f32(i), 0.0).rgb;
    }

    return vec4(sum / vec3(1.0 + f32(support)), 1.0);
}

/// Calculates the horizontal component of the separable Gaussian blur.
@fragment
fn gaussian_horizontal(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coc = calculate_circle_of_confusion(in.position);
    return gaussian_blur(color_texture_a, color_texture_sampler, in.position, coc, vec2(1.0, 0.0));
}

/// Calculates the vertical component of the separable Gaussian blur.
@fragment
fn gaussian_vertical(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coc = calculate_circle_of_confusion(in.position);
    return gaussian_blur(color_texture_a, color_texture_sampler, in.position, coc, vec2(0.0, 1.0));
}

/// Calculates the vertical and first diagonal components of the separable
/// hexagonal bokeh blur.
///
///         ╱
///        ╱
///       •
///       │
///       │
@fragment
fn bokeh_pass_a(in: FullscreenVertexOutput) -> DualOutput {
    let coc = calculate_circle_of_confusion(in.position);
    let vertical = box_blur_a(in.position, coc, vec2(0.0, 1.0));
    let diagonal = box_blur_a(in.position, coc, vec2(COS_NEG_FRAC_PI_6, SIN_NEG_FRAC_PI_6));

    // Note that the diagonal part is pre-mixed with the vertical component.
    var output: DualOutput;
    output.output_0 = vertical;
    output.output_1 = mix(vertical, diagonal, 0.5);
    return output;
}

/// Calculates the second diagonal components of the separable hexagonal bokeh
/// blur.
///
///     ╲   ╱
///      ╲ ╱
///       •
@if(DUAL_INPUT)
@fragment
fn bokeh_pass_b(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coc = calculate_circle_of_confusion(in.position);
    let output_0 = box_blur_a(in.position, coc, vec2(COS_NEG_FRAC_PI_6, SIN_NEG_FRAC_PI_6));
    let output_1 = box_blur_b(in.position, coc, vec2(COS_NEG_FRAC_PI_5_6, SIN_NEG_FRAC_PI_5_6));
    return mix(output_0, output_1, 0.5);
}
