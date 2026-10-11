import bevy_sprite_render::{mesh2d::vertex_output::VertexOutput, sprite_mesh::functions as sprite_fn};

struct DissolveMaterial {
    burn_edge_color: vec4<f32>,
    amount: f32,
    burn_edge: f32,

    // WebGL2 requires structs to be aligned to 16 bytes
    @if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_12b: u32,
    @if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_16b: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(20) var<uniform> material: DissolveMaterial;

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample some simplex noise with the UV coordinates scaled up by 2 to make the noise denser
    var noise = fbm_simplex_2d(input.uv * 2.0);
    // Change the noise's range from 0..1 to burn_edge..1 - burn_edge
    // in order to avoid the burn edge showing up when the sprite is fully visible or fully hidden
    noise = mix(material.burn_edge, 1.0 - material.burn_edge, noise);

    // If this value is 0..burn_edge, the pixel is part of the burn edge
    // If it's >= burn_edge, the pixel is visible
    // Otherwise, it's hidden
    let difference = noise - material.amount;
    // Get the sprite's color
    let sprite_color = sprite_fn::sample_sprite_texture(input.uv, input.instance_index);

    var color = vec4(0.0);
    let burn_edge = difference >= 0 && difference < material.burn_edge;
    let visible = difference >= material.burn_edge;
    if burn_edge {
        color = material.burn_edge_color;
        color.a = sprite_color.a;
    } else if visible {
        color = sprite_color;
    }

    // Apply alpha discard and tint
    color = sprite_fn::get_final_color(color, input.instance_index);
    return color;
}

/// An implementation of the simplex noise algorithm to generate noise for the dissolve effect. Credit to <https://crates.io/crates/noisy_bevy>.
///
/// Note that usually sampling the noise from a prebaked texture is better for performance.
/// However, to avoid increasing the size of the bevy git repository, we generate the noise in the shader.
fn simplex_noise_2d(v: vec2<f32>) -> f32 {
    let C = vec4(
        0.211324865405187, // (3.0 - sqrt(3.0)) / 6.0
        0.366025403784439, // 0.5 * (sqrt(3.0) - 1.0)
        -0.577350269189626, // -1.0 + 2.0 * C.x
        0.024390243902439 // 1.0 / 41.0
    );

    // first corner
    var i = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);

    // other corners
    var i1 = select(vec2(0., 1.), vec2(1., 0.), x0.x > x0.y);
    var x12 = x0.xyxy + C.xxzz - vec4(i1, 0., 0.);

    // permutations
    i = i % vec2(289.);

    let p = permute_3_(permute_3_(i.y + vec3(0., i1.y, 1.)) + i.x + vec3(0., i1.x, 1.));
    var m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3(0.));
    m *= m;
    m *= m;

    // gradients: 41 points uniformly over a line, mapped onto a diamond
    // the ring size, 17*17 = 289, is close to a multiple of 41 (41*7 = 287)
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;

    // normalize gradients implicitly by scaling m
    // approximation of: m *= inversesqrt(a0 * a0 + h * h);
    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));

    // compute final noise value at P
    let g = vec3(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return (130. * dot(m, g) + 1.) / 2.;
}

/// Fractional brownian motion using simplex noise. This approach makes the noise look more detailed.
fn fbm_simplex_2d(pos: vec2<f32>) -> f32 {
    const octaves = 3;
    const lacunarity = 2.0;
    const gain = 0.5;
    // Sum of the geometric series with `octaves` as the amount of terms and `gain` as the factor
    // This is the maximum possible value that the noise can reach
    const max = (1 - pow(gain, octaves)) / (1 - gain);

    var sum = 0.;
    var amplitude = 1.;
    var frequency = 1.;

    for (var i = 0; i < octaves; i+= 1) {
        sum += simplex_noise_2d(pos * frequency) * amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    // Map the noise back to the 0..1 range.
    return sum / max;
}

fn permute_3_(x: vec3<f32>) -> vec3<f32> {
    return (((x * 34.) + 1.) * x) % vec3(289.);
}
