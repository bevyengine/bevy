#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals
#import bevy_sprite::mesh2d_functions

fn permute_3_(x: vec3<f32>) -> vec3<f32> {
    return (((x * 34.) + 1.) * x) % vec3(289.);
}

// Noise implementation from https://github.com/johanhelsing/noisy_bevy/blob/v0.8.0/assets/noisy_bevy.wgsl
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
    return 130. * dot(m, g);
}

fn sum(v: vec4<f32>) -> f32 {
    return v.x+v.y+v.z;
}

// Stochastic sampling method from https://iquilezles.org/articles/texturerepetition/
fn stochastic_sampling(uv: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, s: f32) -> vec4<f32> {

    // sample variation pattern
    let frequency_scale = 5.0;
    let amplitude_scale = 0.3;
    let k = simplex_noise_2d(uv.xy / frequency_scale) * amplitude_scale;

    // compute index from 0-7
    let index = k * 8.0;
    let i = floor(index);
    let f = fract(index);

    // offsets for the different virtual patterns from 0 to 7
    let offa = sin(vec2<f32>(3.0,7.0)*(i+0.0)); // can replace with any other hash
    let offb = sin(vec2<f32>(3.0,7.0)*(i+1.0)); // can replace with any other hash

    // sample the two closest virtual patterns
    let cola = textureSampleGrad(texture, texture_sampler, uv + s * offa, dx, dy);
    let colb = textureSampleGrad(texture, texture_sampler, uv + s * offb, dx, dy);

    // interpolate between the two virtual patterns
    return mix(cola, colb, smoothstep(0.2,0.8,f - 0.1*sum(cola-colb)) );
}

@group(2) @binding(1) var texture: texture_2d<f32>;
@group(2) @binding(2) var texture_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let speed = 0.5;
    let s = smoothstep(0.4, 0.6, sin(globals.time * speed));
    return stochastic_sampling(in.uv, dpdx(in.uv), dpdy(in.uv), s);
}