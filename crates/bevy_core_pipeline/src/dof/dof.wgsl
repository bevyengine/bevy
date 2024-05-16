// Performs depth of field postprocessing, with both Gaussian and bokeh kernels.
//
// Gaussian blur is performed as a separable convolution: first blurring in the
// X direction, and then in the Y direction. This is asymptotically more
// efficient than performing a 2D convolution.
//
// The Bokeh blur uses a similar, but more complex, separable convolution
// technique. The algorithm is described in Colin Barré-Brisebois, "Hexagonal
// Bokeh Blur Revisited" [1]. It's motivated by the observation that we can use
// separable convolutions not only to produce boxes but to produce
// parallelograms. Thus, by performing three separable convolutions in sequence,
// we can produce a hexagonal shape. The first and second convolutions are done
// simultaneously using multiple render targets to cut the total number of
// passes down to two.
//
// [1]: https://colinbarrebrisebois.com/2017/04/18/hexagonal-bokeh-blur-revisited-part-2-improved-2-pass-version/

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::depth_ndc_to_view_z
#import bevy_render::view::View

// Parameters that control the depth of field effect. See
// `bevy_core_pipeline::dof::DepthOfFieldUniforms` for information on what these
// parameters mean.
struct DepthOfFieldParams {
    /// The distance in meters to the location in focus.
    focal_distance: f32,

    /// The [focal length]. Physically speaking, this represents "the distance
    /// from the center of the lens to the principal foci of the lens". The
    /// default value, 50 mm, is considered representative of human eyesight.
    /// Real-world lenses range from anywhere from 5 mm for "fisheye" lenses to
    /// 2000 mm for "super-telephoto" lenses designed for very distant objects.
    ///
    /// The higher the value, the more blurry objects not in focus will be.
    ///
    /// [focal length]: https://en.wikipedia.org/wiki/Focal_length
    focal_length: f32,

    /// The premultiplied factor that we scale the circle of confusion by.
    ///
    /// This is calculated as `focal_length² / (sensor_height * aperture_f_stops)`.
    coc_scale_factor: f32,

    /// The maximum diameter, in pixels, that we allow a circle of confusion to be.
    ///
    /// A circle of confusion essentially describes the size of a blur.
    ///
    /// This value is nonphysical but is useful for avoiding pathologically-slow
    /// behavior.
    max_circle_of_confusion_diameter: f32,

    /// The depth value that we clamp distant objects to. See the comment in
    /// [`DepthOfFieldSettings`] for more information.
    max_depth: f32,

    /// Padding.
    pad_a: u32,
    /// Padding.
    pad_b: u32,
    /// Padding.
    pad_c: u32,
}

// The first bokeh pass outputs to two render targets. We declare them here.
struct DualOutput {
    // The vertical output.
    @location(0) output_0: vec4<f32>,
    // The diagonal output.
    @location(1) output_1: vec4<f32>,
}

// @group(0) @binding(0) is `mesh_view_bindings::view`.

// The depth texture for the main view.
#ifdef MULTISAMPLED
@group(0) @binding(1) var depth_texture: texture_depth_multisampled_2d;
#else   // MULTISAMPLED
@group(0) @binding(1) var depth_texture: texture_depth_2d;
#endif  // MULTISAMPLED

// The main color texture.
@group(0) @binding(2) var color_texture_a: texture_2d<f32>;

// The auxiliary color texture that we're sampling from. This is only used as
// part of the second bokeh pass.
#ifdef DUAL_INPUT
@group(0) @binding(3) var color_texture_b: texture_2d<f32>;
#endif  // DUAL_INPUT

// The global uniforms, representing data backed by buffers shared among all
// views in the scene.

// The parameters that control the depth of field effect.
@group(1) @binding(0) var<uniform> dof_params: DepthOfFieldParams;

// The sampler that's used to fetch texels from the source color buffer.
@group(1) @binding(1) var color_texture_sampler: sampler;

// cos(-30°), used for the bokeh blur.
const COS_NEG_FRAC_PI_6: f32 = 0.8660254037844387;
// sin(-30°), used for the bokeh blur.
const SIN_NEG_FRAC_PI_6: f32 = -0.5;
// cos(-150°), used for the bokeh blur.
const COS_NEG_FRAC_PI_5_6: f32 = -0.8660254037844387;
// sin(-150°), used for the bokeh blur.
const SIN_NEG_FRAC_PI_5_6: f32 = -0.5;

// Calculates and returns the diameter (not the radius) of the [circle of
// confusion].
//
// [circle of confusion]: https://en.wikipedia.org/wiki/Circle_of_confusion
fn calculate_circle_of_confusion(in_frag_coord: vec4<f32>) -> f32 {
    // Unpack the depth of field parameters.
    let focus = dof_params.focal_distance;
    let f = dof_params.focal_length;
    let scale = dof_params.coc_scale_factor;
    let max_coc_diameter = dof_params.max_circle_of_confusion_diameter;

    // Sample the depth.
    let frag_coord = vec2<i32>(floor(in_frag_coord.xy));
    let raw_depth = textureLoad(depth_texture, frag_coord, 0);
    let depth = min(-depth_ndc_to_view_z(raw_depth), dof_params.max_depth);

    // Calculate the circle of confusion.
    //
    // This is just the formula from Wikipedia [1].
    //
    // [1]: https://en.wikipedia.org/wiki/Circle_of_confusion#Determining_a_circle_of_confusion_diameter_from_the_object_field
    let candidate_coc = scale * abs(depth - focus) / (depth * (focus - f));

    let framebuffer_size = vec2<f32>(textureDimensions(color_texture_a));
    return clamp(candidate_coc * framebuffer_size.y, 0.0, max_coc_diameter);
}

// Performs a single direction of the separable Gaussian blur kernel.
//
// * `frag_coord` is the screen-space pixel coordinate of the fragment (i.e. the
//   `position` input to the fragment).
//
// * `coc` is the diameter (not the radius) of the circle of confusion for this
//   fragment.
//
// * `frag_offset` is the vector, in screen-space units, from one sample to the
//   next. For a horizontal blur this will be `vec2(1.0, 0.0)`; for a vertical
//   blur this will be `vec2(0.0, 1.0)`.
//
// Returns the resulting color of the fragment.
fn gaussian_blur(frag_coord: vec4<f32>, coc: f32, frag_offset: vec2<f32>) -> vec4<f32> {
    // Usually σ (the standard deviation) is half the radius, and the radius is
    // half the CoC. So we multiply by 0.25.
    let sigma = coc * 0.25;

    // 1.5σ is a good, somewhat aggressive default for support—the number of
    // texels on each side of the center that we process.
    let support = i32(ceil(sigma * 1.5));
    let uv = frag_coord.xy / vec2<f32>(textureDimensions(color_texture_a));
    let offset = frag_offset / vec2<f32>(textureDimensions(color_texture_a));

    // The probability density function of the Gaussian blur is (up to constant factors) `exp(-1 / 2σ² *
    // x²). We precalculate the constant factor here to avoid having to
    // calculate it in the inner loop.
    let exp_factor = -1.0 / (2.0 * sigma * sigma);

    // Accumulate samples on both sides of the current texel. Go two at a time,
    // taking advantage of bilinear filtering.
    var sum = textureSampleLevel(color_texture_a, color_texture_sampler, uv, 0.0).rgb;
    var weight_sum = 1.0;
    for (var i = 1; i <= support; i += 2) {
        // This is a well-known trick to reduce the number of needed texture
        // samples by a factor of two. We seek to accumulate two adjacent
        // samples c₀ and c₁ with weights w₀ and w₁ respectively, with a single
        // texture sample at a carefully chosen location. Observe that:
        //
        //     k ⋅ lerp(c₀, c₁, t) = w₀⋅c₀ + w₁⋅c₁
        //
        //                              w₁
        //     if k = w₀ + w₁ and t = ───────
        //                            w₀ + w₁
        //
        // Therefore, if we sample at a distance of t = w₁ / (w₀ + w₁) texels in
        // between the two texel centers and scale by k = w₀ + w₁ afterward, we
        // effectively evaluate w₀⋅c₀ + w₁⋅c₁ with a single texture lookup.
        let w0 = exp(exp_factor * f32(i) * f32(i));
        let w1 = exp(exp_factor * f32(i + 1) * f32(i + 1));
        let uv_offset = offset * (f32(i) + w1 / (w0 + w1));
        let weight = w0 + w1;

        sum += (
            textureSampleLevel(color_texture_a, color_texture_sampler, uv + uv_offset, 0.0).rgb +
            textureSampleLevel(color_texture_a, color_texture_sampler, uv - uv_offset, 0.0).rgb
        ) * weight;
        weight_sum += weight * 2.0;
    }

    return vec4(sum / weight_sum, 1.0);
}

// Performs a box blur in a single direction, sampling `color_texture_a`.
//
// * `frag_coord` is the screen-space pixel coordinate of the fragment (i.e. the
//   `position` input to the fragment).
//
// * `coc` is the diameter (not the radius) of the circle of confusion for this
//   fragment.
//
// * `frag_offset` is the vector, in screen-space units, from one sample to the
//   next. This need not be horizontal or vertical.
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

// Performs a box blur in a single direction, sampling `color_texture_b`.
//
// * `frag_coord` is the screen-space pixel coordinate of the fragment (i.e. the
//   `position` input to the fragment).
//
// * `coc` is the diameter (not the radius) of the circle of confusion for this
//   fragment.
//
// * `frag_offset` is the vector, in screen-space units, from one sample to the
//   next. This need not be horizontal or vertical.
#ifdef DUAL_INPUT
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
#endif

// Calculates the horizontal component of the separable Gaussian blur.
@fragment
fn gaussian_horizontal(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coc = calculate_circle_of_confusion(in.position);
    return gaussian_blur(in.position, coc, vec2(1.0, 0.0));
}

// Calculates the vertical component of the separable Gaussian blur.
@fragment
fn gaussian_vertical(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coc = calculate_circle_of_confusion(in.position);
    return gaussian_blur(in.position, coc, vec2(0.0, 1.0));
}

// Calculates the vertical and first diagonal components of the separable
// hexagonal bokeh blur.
//
//         ╱
//        ╱
//       •
//       │
//       │
@fragment
fn bokeh_pass_0(in: FullscreenVertexOutput) -> DualOutput {
    let coc = calculate_circle_of_confusion(in.position);
    let vertical = box_blur_a(in.position, coc, vec2(0.0, 1.0));
    let diagonal = box_blur_a(in.position, coc, vec2(COS_NEG_FRAC_PI_6, SIN_NEG_FRAC_PI_6));

    // Note that the diagonal part is pre-mixed with the vertical component.
    var output: DualOutput;
    output.output_0 = vertical;
    output.output_1 = mix(vertical, diagonal, 0.5);
    return output;
}

// Calculates the second diagonal components of the separable hexagonal bokeh
// blur.
//
//     ╲   ╱
//      ╲ ╱
//       •
#ifdef DUAL_INPUT
@fragment
fn bokeh_pass_1(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coc = calculate_circle_of_confusion(in.position);
    let output_0 = box_blur_a(in.position, coc, vec2(COS_NEG_FRAC_PI_6, SIN_NEG_FRAC_PI_6));
    let output_1 = box_blur_b(in.position, coc, vec2(COS_NEG_FRAC_PI_5_6, SIN_NEG_FRAC_PI_5_6));
    return mix(output_0, output_1, 0.5);
}
#endif
