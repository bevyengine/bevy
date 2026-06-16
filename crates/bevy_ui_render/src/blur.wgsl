// Blur algorithms for blur regions behind UI nodes.
//
// Every pass shares the same uniform. `params` is interpreted per algorithm:
//
//   Box blur:    x = kernel radius in pixels, y = sample spacing multiplier
//   Gaussian:    x = circle of confusion diameter in pixels, y = sigma multiplier
//   Dual kawase: x = sample offset multiplier
//   Bokeh:       x = kernel radius in pixels, y = 1 / kernel normalization factor

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_ui::ui_node::sd_rounded_box

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> blur_regions: BlurRegionUniform;
// Auxiliary inputs, only bound for the passes that use them.
// Kawase composite: the half-resolution blurred scene.
// Bokeh vertical:   the complex response of the red and green channels.
@group(0) @binding(3) var aux_texture_a: texture_2d<f32>;
// Bokeh vertical: the complex response of the blue channel.
@group(0) @binding(4) var aux_texture_b: texture_2d<f32>;

struct BlurRegionUniform {
    params: vec4<f32>,
    current_regions_count: u32,
    regions: array<ComputedBlurRegion, #{MAX_BLUR_REGIONS_COUNT}>,
}

struct ComputedBlurRegion {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    border_radii: vec4<f32>,
}

// Whether a pixel falls inside any blur region, so that passes can skip the
// expensive blur work for pixels that don't need it.
fn is_blurred(position: vec2<f32>) -> bool {
    return min_region_distance(position) <= 0.0;
}

// Returns the signed distance, in pixels, from `position` to the closest blur
// region. Negative inside a region, positive outside.
fn min_region_distance(position: vec2<f32>) -> f32 {
    var min_distance = 1e30;
    for (var i = 0u; i < blur_regions.current_regions_count; i++) {
        let region = blur_regions.regions[i];
        let center = vec2((region.max_x + region.min_x) * 0.5, (region.max_y + region.min_y) * 0.5);
        let dims = vec2(region.max_x - region.min_x, region.max_y - region.min_y);
        let half_smallest_dimension = min(dims.x, dims.y) * 0.5;

        let distance = sd_rounded_box(
            position - center,
            dims,
            min(region.border_radii, vec4(half_smallest_dimension)),
        );
        min_distance = min(min_distance, distance);
    }
    return min_distance;
}

// ---------------------------------------------------------------------------
// Gaussian blur
// ---------------------------------------------------------------------------
// Performs a single direction of the separable Gaussian blur kernel.
//
// * `frag_coord` is the screen-space pixel coordinate of the fragment.
//
// * `frag_offset` is the vector, in screen-space units, from one sample to the
//   next: `vec2(1.0, 0.0)` for the horizontal pass and `vec2(0.0, 1.0)` for the
//   vertical pass.
fn gaussian_blur(frag_coord: vec4<f32>, frag_offset: vec2<f32>) -> vec4<f32> {
    // The standard deviation as a fraction of the circle of confusion. Usually σ
    // is half the radius, and the radius is half the CoC, giving the default 0.25.
    let coc = blur_regions.params.x;
    let sigma = coc * blur_regions.params.y;

    // 1.5σ is a good, somewhat aggressive default for support—the number of
    // texels on each side of the center that we process.
    let support = i32(ceil(sigma * 1.5));
    let uv = frag_coord.xy / vec2<f32>(textureDimensions(input_texture));
    let offset = frag_offset / vec2<f32>(textureDimensions(input_texture));

    // The probability density function of the Gaussian blur is (up to constant factors) `exp(-1 / 2σ² *
    // x²). We precalculate the constant factor here to avoid having to
    // calculate it in the inner loop.
    let exp_factor = -1.0 / (2.0 * sigma * sigma);

    // Accumulate samples on both sides of the current texel. Go two at a time,
    // taking advantage of bilinear filtering.
    var sum = textureSampleLevel(input_texture, texture_sampler, uv, 0.0).rgb;
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
            textureSampleLevel(input_texture, texture_sampler, uv + uv_offset, 0.0).rgb +
            textureSampleLevel(input_texture, texture_sampler, uv - uv_offset, 0.0).rgb
        ) * weight;
        weight_sum += weight * 2.0;
    }

    return vec4(sum / weight_sum, 1.0);
}

@fragment
fn gaussian_horizontal(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if !is_blurred(in.position.xy) {
        return textureSampleLevel(input_texture, texture_sampler, in.uv, 0.0);
    }
    return gaussian_blur(in.position, vec2(1.0, 0.0));
}

@fragment
fn gaussian_vertical(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if !is_blurred(in.position.xy) {
        return textureSampleLevel(input_texture, texture_sampler, in.uv, 0.0);
    }
    return gaussian_blur(in.position, vec2(0.0, 1.0));
}

// ---------------------------------------------------------------------------
// Box blur
// ---------------------------------------------------------------------------
// Performs a single direction of the separable box blur kernel: a plain average
// of `2 * radius + 1` samples spaced `scale` pixels apart along `frag_offset`.
fn box_blur(frag_coord: vec4<f32>, frag_offset: vec2<f32>) -> vec4<f32> {
    let radius = i32(blur_regions.params.x);
    let scale = blur_regions.params.y;

    let uv = frag_coord.xy / vec2<f32>(textureDimensions(input_texture));
    let offset = frag_offset * scale / vec2<f32>(textureDimensions(input_texture));

    var sum = textureSampleLevel(input_texture, texture_sampler, uv, 0.0).rgb;
    for (var i = 1; i <= radius; i++) {
        sum += textureSampleLevel(input_texture, texture_sampler, uv + offset * f32(i), 0.0).rgb;
        sum += textureSampleLevel(input_texture, texture_sampler, uv - offset * f32(i), 0.0).rgb;
    }

    return vec4(sum / f32(2 * radius + 1), 1.0);
}

@fragment
fn box_horizontal(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if !is_blurred(in.position.xy) {
        return textureSampleLevel(input_texture, texture_sampler, in.uv, 0.0);
    }
    return box_blur(in.position, vec2(1.0, 0.0));
}

@fragment
fn box_vertical(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if !is_blurred(in.position.xy) {
        return textureSampleLevel(input_texture, texture_sampler, in.uv, 0.0);
    }
    return box_blur(in.position, vec2(0.0, 1.0));
}

// ---------------------------------------------------------------------------
// Dual kawase blur
// ---------------------------------------------------------------------------
// The scene is downsampled through a chain of progressively half-resolution
// textures and then upsampled back, blurring a little at every step. Because
// most passes run at reduced resolution this produces very wide, smooth blurs
// far cheaper than a single-pass kernel of equivalent width. Blur regions are
// only applied in the final composite pass, which picks per pixel between the
// blurred chain and the untouched scene.

// Downsamples to half resolution: a center sample weighted 4 plus the four
// diagonal neighbors. `half_pixel` of the destination equals one source texel.
@fragment
fn kawase_downsample(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let offset = blur_regions.params.x;
    let half_pixel = offset / vec2<f32>(textureDimensions(input_texture));

    var sum = textureSampleLevel(input_texture, texture_sampler, in.uv, 0.0).rgb * 4.0;
    sum += textureSampleLevel(input_texture, texture_sampler, in.uv - half_pixel, 0.0).rgb;
    sum += textureSampleLevel(input_texture, texture_sampler, in.uv + half_pixel, 0.0).rgb;
    sum += textureSampleLevel(input_texture, texture_sampler, in.uv + vec2(half_pixel.x, -half_pixel.y), 0.0).rgb;
    sum += textureSampleLevel(input_texture, texture_sampler, in.uv - vec2(half_pixel.x, -half_pixel.y), 0.0).rgb;

    return vec4(sum / 8.0, 1.0);
}

// Upsamples `source` to double resolution with the 8-tap dual kawase pattern.
// `half_pixel` of the destination equals a quarter of a source texel.
fn kawase_upsample_from(source: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let offset = blur_regions.params.x;
    let half_pixel = 0.25 * offset / vec2<f32>(textureDimensions(source));

    var sum = textureSampleLevel(source, texture_sampler, uv + vec2(-half_pixel.x * 2.0, 0.0), 0.0).rgb;
    sum += textureSampleLevel(source, texture_sampler, uv + vec2(-half_pixel.x, half_pixel.y), 0.0).rgb * 2.0;
    sum += textureSampleLevel(source, texture_sampler, uv + vec2(0.0, half_pixel.y * 2.0), 0.0).rgb;
    sum += textureSampleLevel(source, texture_sampler, uv + vec2(half_pixel.x, half_pixel.y), 0.0).rgb * 2.0;
    sum += textureSampleLevel(source, texture_sampler, uv + vec2(half_pixel.x * 2.0, 0.0), 0.0).rgb;
    sum += textureSampleLevel(source, texture_sampler, uv + vec2(half_pixel.x, -half_pixel.y), 0.0).rgb * 2.0;
    sum += textureSampleLevel(source, texture_sampler, uv + vec2(0.0, -half_pixel.y * 2.0), 0.0).rgb;
    sum += textureSampleLevel(source, texture_sampler, uv + vec2(-half_pixel.x, -half_pixel.y), 0.0).rgb * 2.0;

    return vec4(sum / 12.0, 1.0);
}

@fragment
fn kawase_upsample(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return kawase_upsample_from(input_texture, in.uv);
}

// The final upsample back to full resolution, masked to the blur regions.
// `input_texture` is the untouched scene and `aux_texture_a` is the
// half-resolution blurred chain.
@fragment
fn kawase_composite(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if !is_blurred(in.position.xy) {
        return textureSampleLevel(input_texture, texture_sampler, in.uv, 0.0);
    }
    return kawase_upsample_from(aux_texture_a, in.uv);
}

// ---------------------------------------------------------------------------
// Bokeh blur
// ---------------------------------------------------------------------------
// A separable convolution with a single-component complex Gaussian kernel
//
//     c(x) = exp(-a·x²) · (cos(b·x²) + i·sin(b·x²))
//
// whose 2D self-product has a magnitude close to a flat disc, mimicking the
// aperture of a camera. Parameters are drawn from <http://yehar.com/blog/?p=1495>
// via <https://github.com/mikepound/convolve/blob/master/complex_kernels.py>.
//
// The horizontal pass convolves each color channel with the complex kernel and
// stores the complex responses across two textures. The vertical pass finishes
// the separable convolution with complex multiplies and resolves the result to
// a real color as `A·real + B·imag`, normalized by a factor computed on the CPU
// (`bokeh_normalization` in `blur.rs`).
//
// These constants must stay in sync with their counterparts in `blur.rs`.
const BOKEH_KERNEL_A: f32 = 0.862325;
const BOKEH_KERNEL_B: f32 = 1.624835;
const BOKEH_WEIGHT_REAL: f32 = 0.767583;
const BOKEH_WEIGHT_IMAG: f32 = 1.862321;
const BOKEH_KERNEL_SCALE: f32 = 1.4;

// Evaluates the complex kernel at `x` texels from the center as (real, imaginary).
fn bokeh_kernel(x: f32, radius: f32) -> vec2<f32> {
    let t = x * BOKEH_KERNEL_SCALE / radius;
    let t2 = t * t;
    let magnitude = exp(-BOKEH_KERNEL_A * t2);
    return vec2(magnitude * cos(BOKEH_KERNEL_B * t2), magnitude * sin(BOKEH_KERNEL_B * t2));
}

fn complex_multiply(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

struct BokehHorizontalOutput {
    // The complex response of the red and green channels: (R.re, R.im, G.re, G.im).
    @location(0) red_green: vec4<f32>,
    // The complex response of the blue channel: (B.re, B.im, 0, 0).
    @location(1) blue: vec4<f32>,
}

@fragment
fn bokeh_horizontal(in: FullscreenVertexOutput) -> BokehHorizontalOutput {
    var out: BokehHorizontalOutput;
    out.red_green = vec4(0.0);
    out.blue = vec4(0.0);

    // The vertical pass only reads this texture within `radius` pixels of a blur
    // region, so everything further away can be skipped.
    let radius = blur_regions.params.x;
    if min_region_distance(in.position.xy) > radius {
        return out;
    }

    let texel = 1.0 / vec2<f32>(textureDimensions(input_texture));
    let support = i32(radius);
    for (var x = -support; x <= support; x++) {
        let kernel = bokeh_kernel(f32(x), radius);
        let color = textureSampleLevel(
            input_texture, texture_sampler, in.uv + vec2(f32(x), 0.0) * texel, 0.0).rgb;
        // The input color is real-valued, so this is a scalar-complex product.
        out.red_green += vec4(color.r * kernel, color.g * kernel);
        out.blue += vec4(color.b * kernel, 0.0, 0.0);
    }

    return out;
}

@fragment
fn bokeh_vertical(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if !is_blurred(in.position.xy) {
        return textureSampleLevel(input_texture, texture_sampler, in.uv, 0.0);
    }

    let radius = blur_regions.params.x;
    let inverse_normalization = blur_regions.params.y;
    let texel = 1.0 / vec2<f32>(textureDimensions(aux_texture_a));

    var red = vec2(0.0);
    var green = vec2(0.0);
    var blue = vec2(0.0);

    let support = i32(radius);
    for (var y = -support; y <= support; y++) {
        let kernel = bokeh_kernel(f32(y), radius);
        let uv = in.uv + vec2(0.0, f32(y)) * texel;
        let red_green = textureSampleLevel(aux_texture_a, texture_sampler, uv, 0.0);
        let blue_sample = textureSampleLevel(aux_texture_b, texture_sampler, uv, 0.0);
        red += complex_multiply(red_green.xy, kernel);
        green += complex_multiply(red_green.zw, kernel);
        blue += complex_multiply(blue_sample.xy, kernel);
    }

    // Resolve the complex accumulators to a real color.
    let weights = vec2(BOKEH_WEIGHT_REAL, BOKEH_WEIGHT_IMAG);
    let color = vec3(dot(red, weights), dot(green, weights), dot(blue, weights))
        * inverse_normalization;

    // The kernel has small negative lobes, so the reconstruction rings into
    // negative, out-of-gamut values around hard HDR edges. Desaturate toward the
    // (non-negative) luminance just enough to lift the most-negative channel to 
    // zero. This preserves luminance and keeps the hue stable. 

    // The constants for calculating luminance are from the BT.709 standard.
    let bt709 : vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);
    
    let luma = max(dot(color, bt709), 0.0);
    let min_channel = min(color.r, min(color.g, color.b));
    var resolved = color;
    if min_channel < 0.0 {
        // Solve luma + t * (min_channel - luma) = 0 for the blend toward gray.
        let t = luma / (luma - min_channel);
        resolved = mix(vec3(luma), color, t);
    }
    // Guard against any residual negatives from floating-point error.
    return vec4(max(resolved, vec3(0.0)), 1.0);
}
