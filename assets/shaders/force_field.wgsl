#import bevy_pbr::{
    mesh_view_bindings::{globals, view},
    prepass_utils,
    forward_io::VertexOutput,
}

@fragment
fn fragment(
    #ifdef MULTISAMPLED
    @builtin(sample_index) sample_index: u32,
    #endif
    @builtin(position) frag_coord: vec4<f32>,
    @builtin(front_facing) is_front: bool,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    let emissive_intensity = 20.0;
    let fresnel_color = vec3(0.5, 1.0, 1.0);
    var intersection_color = fresnel_color;
    let offset = 0.92;
    let fresnel_exp = 5.0;
    let intersection_intensity = 10.0;
    let noise_scale = 4.0;
    let time_scale = 0.2;
    let time = globals.time * time_scale;


    let depth = bevy_pbr::prepass_utils::prepass_depth(frag_coord, sample_index);
    var intersection = 1.0 - ((frag_coord.z - depth) * 100.0) - offset;
    intersection = smoothstep(0.0, 1.0, intersection);
    if is_front{
        intersection *= intersection_intensity;
    } else {
        intersection *= intersection_intensity / 2.0;
    }

    let V = normalize(view.world_position.xyz - world_position.xyz);
    var fresnel = 1.0 - dot(world_normal, V);
    fresnel = pow(fresnel, fresnel_exp);

    var a = 0.0;

    a += fbm(uv * noise_scale + vec2(time));
    a += fbm(uv * noise_scale - vec2(time));
    a = clamp(a, 0.0, 1.0);

    a += intersection;
    if is_front {
        a += fresnel;
    }

    var color = intersection * intersection_color;
    if is_front {
        color += fresnel * fresnel_color;
    }

    color *= emissive_intensity;

    if all(color <= vec3(1.0)) {
        color += fresnel_color/emissive_intensity;
    }

    return vec4(color * a, 0.0);
}


fn mod289(x: vec2<f32>) -> vec2<f32> {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn mod289_3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn permute3(x: vec3<f32>) -> vec3<f32> {
    return mod289_3(((x * 34.) + 1.) * x);
}

//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket
fn simplexNoise2(v: vec2<f32>) -> f32 {
    let C = vec4(
        0.211324865405187, // (3.0-sqrt(3.0))/6.0
        0.366025403784439, // 0.5*(sqrt(3.0)-1.0)
        -0.577350269189626, // -1.0 + 2.0 * C.x
        0.024390243902439 // 1.0 / 41.0
    );

    // First corner
    var i = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);

    // Other corners
    var i1 = select(vec2(0., 1.), vec2(1., 0.), x0.x > x0.y);

    // x0 = x0 - 0.0 + 0.0 * C.xx ;
    // x1 = x0 - i1 + 1.0 * C.xx ;
    // x2 = x0 - 1.0 + 2.0 * C.xx ;
    var x12 = x0.xyxy + C.xxzz;
    x12.x = x12.x - i1.x;
    x12.y = x12.y - i1.y;

    // Permutations
    i = mod289(i); // Avoid truncation effects in permutation

    var p = permute3(permute3(i.y + vec3(0., i1.y, 1.)) + i.x + vec3(0., i1.x, 1.));
    var m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3(0.));
    m = pow(m, vec3(4.));

    // Gradients: 41 points uniformly over a line, mapped onto a diamond.
    // The ring size 17*17 = 289 is close to a multiple of 41 (41*7 = 287)
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;

    // Normalize gradients implicitly by scaling m
    // Approximation of: m *= inversesqrt( a0*a0 + h*h );
    m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);

    // Compute final noise value at P
    let g = vec3(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return 130. * dot(m, g);
}

fn noise2(n: vec2<f32>) -> f32 {
   return simplexNoise2(n);
}

//  MIT License. © Inigo Quilez, Munrocket
//
fn fbm(p: vec2<f32>) -> f32 {
    return fbm2(p, 1.0, 4u);
}


fn fbm2(x: vec2<f32>, H: f32, numOctaves: u32) -> f32 {
    var G = exp2(-H);
    var f = 1.0;
    var a = 1.0;
    var t = 0.0;
    for(var i = 0u; i < numOctaves; i++ ) {
        t += a*noise2(f*x);
        f *= 2.0;
        a *= G;
    }
    return t;
}