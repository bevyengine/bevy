// The film grain postprocessing effect.

#define_import_path bevy_post_process::effect_stack::film_grain

#import bevy_post_process::effect_stack::bindings::{source_texture, film_grain_texture, film_grain_sampler, film_grain_settings, FilmGrainSettings, EPSILON}

// Reference: https://www.shadertoy.com/view/4tXyWN
fn hash(p: vec2<u32>) -> f32 {
    var p_mut = p;
    p_mut *= vec2<u32>(73333, 7777);
    p_mut.x ^= 3333777777u >> (p_mut.x >> 28u);
    p_mut.y ^= 3333777777u >> (p_mut.y >> 28u);

    let n = p_mut.x * p_mut.y;
    let h = n ^ (n >> 15u);
    return f32(h) * (1.0/4294967296.0);
}

fn get_rgb_noise(grid_x: i32, grid_y: i32, frame_offset: u32) -> vec3<f32> {
    let coord = vec2<u32>(vec2<i32>(grid_x, grid_y)) + frame_offset;
    let hash_val = hash(coord);
    let r = hash_val;
    // Derive uncorrelated random values for G and B channels from the
    // single base hash. This avoids the computational cost of calling
    // hash() multiple times.
    let g = fract(hash_val * 12.9898 + 78.233);
    let b = fract(hash_val * 63.346 + 45.543);
    return vec3<f32>(r, g, b);
}

fn get_procedural_grain(uv: vec2<f32>, screen_dimensions: vec2<u32>, grain_size: f32) -> vec3<f32> {
    // Convert UV to pixel coordinates, then scale down by grain size.
    // This creates a virtual grid where each cell represents a grain chunk.
    let pixel_coord = uv * vec2<f32>(screen_dimensions);
    let scaled_coord = pixel_coord / grain_size;

    // Calculate a frame offset based on screen resolution to animate the grain.
    let frame_offset = film_grain_settings.frame * screen_dimensions.x * screen_dimensions.y;

    // Split coordinates into integer (grid cell) and fractional (intra-cell) parts.
    let i = floor(scaled_coord);
    let f = fract(scaled_coord);

    // Sample noise at the 4 corners of the current grid cell.
    let v00 = get_rgb_noise(i32(i.x), i32(i.y), frame_offset);
    let v10 = get_rgb_noise(i32(i.x) + 1, i32(i.y), frame_offset);
    let v01 = get_rgb_noise(i32(i.x), i32(i.y) + 1, frame_offset);
    let v11 = get_rgb_noise(i32(i.x) + 1, i32(i.y) + 1, frame_offset);

    let u = smoothstep(0.0, 1.0, f.x);
    let v = smoothstep(0.0, 1.0, f.y);

    let mix_x1 = mix(v00, v10, u);
    let mix_x2 = mix(v01, v11, v);

    let r = mix(mix_x1.r, mix_x2.r, f.y);
    let g = mix(mix_x1.g, mix_x2.g, f.y);
    let b = mix(mix_x1.b, mix_x2.b, f.y);

    return vec3<f32>(r, g, b);
}

fn get_grain_sample(uv: vec2<f32>, grain_size: f32) -> vec3<f32> {
    let screen_dimensions = textureDimensions(source_texture);
    let grain_texture_size = vec2<f32>(textureDimensions(film_grain_texture));

    if (grain_texture_size.x < 2 || grain_texture_size.y < 2) {
        return get_procedural_grain(uv, screen_dimensions, grain_size);
    }

    let tiling = vec2<f32>(screen_dimensions) / (grain_texture_size * grain_size);

    // Generate a random offset for each frame to prevent static patterns.
    // We use 101u and 211u as distinct seeds for X and Y axes.
    let rand_x = hash(vec2<u32>(film_grain_settings.frame, 101u));
    let rand_y = hash(vec2<u32>(film_grain_settings.frame, 211u));
    let random_offset = vec2<f32>(rand_x, rand_y);

    let centered_uv = (uv - 0.5) * tiling;
    let final_uv = centered_uv + random_offset + 0.5;

    return textureSample(film_grain_texture, film_grain_sampler, final_uv).rgb;
}

fn film_grain(uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    if (film_grain_settings.intensity < EPSILON) {
        return color;
    }

    let intensity = saturate(film_grain_settings.intensity);
    let shadows_intensity = saturate(film_grain_settings.shadows_intensity);
    let midtones_intensity = saturate(film_grain_settings.midtones_intensity);
    let highlights_intensity = saturate(film_grain_settings.highlights_intensity);
    let shadows_threshold = saturate(film_grain_settings.shadows_threshold);
    let highlights_threshold = saturate(film_grain_settings.highlights_threshold);
    let grain_size = max(film_grain_settings.grain_size, EPSILON);

    // Sample the grain texture (or generate procedural noise).
    let grain_color = get_grain_sample(uv, grain_size);

    // Calculate perceptual luminance using the Rec.709 standard.
    let luminance = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    // Calculate blending factors for shadows, midtones, and highlights.
    let shadow_factor = 1.0 - smoothstep(shadows_threshold - 0.1, shadows_threshold + 0.1, luminance);
    let highlight_factor = smoothstep(highlights_threshold - 0.1, highlights_threshold + 0.1, luminance);
    let midtone_factor = 1.0 - shadow_factor - highlight_factor;

    let local_intensity =
        (shadow_factor * shadows_intensity) +
        (midtone_factor * midtones_intensity) +
        (highlight_factor * highlights_intensity);

    let strength = local_intensity * intensity;
    let overlay_grain = grain_color * strength;
    return color.rgb + overlay_grain;
}
