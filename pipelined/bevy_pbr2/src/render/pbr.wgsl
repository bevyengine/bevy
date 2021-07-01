// TODO: try merging this block with the binding?
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var view: View;


[[block]]
struct Mesh {
    transform: mat4x4<f32>;
};
[[group(1), binding(0)]]
var mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.transform * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.uv = vertex.uv;
    out.world_position = world_position;
    out.clip_position = view.view_proj * world_position;
    // FIXME: The inverse transpose of the model matrix should be used to correctly handle scaling
    // of normals
    out.world_normal = mat3x3<f32>(mesh.transform.x.xyz, mesh.transform.y.xyz, mesh.transform.z.xyz) * vertex.normal;
    return out;
}

// From the Filament design doc
// https://google.github.io/filament/Filament.html#table_symbols
// Symbol Definition
// v    View unit vector
// l    Incident light unit vector
// n    Surface normal unit vector
// h    Half unit vector between l and v
// f    BRDF
// f_d    Diffuse component of a BRDF
// f_r    Specular component of a BRDF
// α    Roughness, remapped from using input perceptualRoughness
// σ    Diffuse reflectance
// Ω    Spherical domain
// f0    Reflectance at normal incidence
// f90    Reflectance at grazing angle
// χ+(a)    Heaviside function (1 if a>0 and 0 otherwise)
// nior    Index of refraction (IOR) of an interface
// ⟨n⋅l⟩    Dot product clamped to [0..1]
// ⟨a⟩    Saturated value (clamped to [0..1])

// The Bidirectional Reflectance Distribution Function (BRDF) describes the surface response of a standard material
// and consists of two components, the diffuse component (f_d) and the specular component (f_r):
// f(v,l) = f_d(v,l) + f_r(v,l)
//
// The form of the microfacet model is the same for diffuse and specular
// f_r(v,l) = f_d(v,l) = 1 / { |n⋅v||n⋅l| } ∫_Ω D(m,α) G(v,l,m) f_m(v,l,m) (v⋅m) (l⋅m) dm
//
// In which:
// D, also called the Normal Distribution Function (NDF) models the distribution of the microfacets
// G models the visibility (or occlusion or shadow-masking) of the microfacets
// f_m is the microfacet BRDF and differs between specular and diffuse components
//
// The above integration needs to be approximated.

[[block]]
struct StandardMaterial {
    base_color: vec4<f32>;
    emissive: vec4<f32>;
    perceptual_roughness: f32;
    metallic: f32;
    reflectance: f32;
    // 'flags' is a bit field indicating various option. uint is 32 bits so we have up to 32 options.
    flags: u32;
};

struct PointLight {
    color: vec4<f32>;
    // projection: mat4x4<f32>;
    position: vec3<f32>;
    inverse_square_range: f32;
    radius: f32;
    near: f32;
    far: f32;
};

[[block]]
struct Lights {
    // NOTE: this array size must be kept in sync with the constants defined bevy_pbr2/src/render/light.rs
    // TODO: this can be removed if we move to storage buffers for light arrays
    point_lights: array<PointLight, 10>;
    ambient_color: vec4<f32>;
    num_lights: u32;
};

let FLAGS_BASE_COLOR_TEXTURE_BIT: u32         = 1u;
let FLAGS_EMISSIVE_TEXTURE_BIT: u32           = 2u;
let FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT: u32 = 4u;
let FLAGS_OCCLUSION_TEXTURE_BIT: u32          = 8u;
let FLAGS_DOUBLE_SIDED_BIT: u32               = 16u;
let FLAGS_UNLIT_BIT: u32                      = 32u;


[[group(0), binding(1)]]
var lights: Lights;
[[group(0), binding(2)]]
var shadow_textures: texture_depth_cube_array;
[[group(0), binding(3)]]
var shadow_textures_sampler: sampler_comparison;

[[group(2), binding(0)]]
var material: StandardMaterial;
[[group(2), binding(1)]]
var base_color_texture: texture_2d<f32>;
[[group(2), binding(2)]]
var base_color_sampler: sampler;
[[group(2), binding(3)]]
var emissive_texture: texture_2d<f32>;
[[group(2), binding(4)]]
var emissive_sampler: sampler;
[[group(2), binding(5)]]
var metallic_roughness_texture: texture_2d<f32>;
[[group(2), binding(6)]]
var metallic_roughness_sampler: sampler;
[[group(2), binding(7)]]
var occlusion_texture: texture_2d<f32>;
[[group(2), binding(8)]]
var occlusion_sampler: sampler;

let PI: f32 = 3.141592653589793;

fn saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

// distanceAttenuation is simply the square falloff of light intensity
// combined with a smooth attenuation at the edge of the light radius
//
// light radius is a non-physical construct for efficiency purposes,
// because otherwise every light affects every fragment in the scene
fn getDistanceAttenuation(distanceSquare: f32, inverseRangeSquared: f32) -> f32 {
    let factor = distanceSquare * inverseRangeSquared;
    let smoothFactor = saturate(1.0 - factor * factor);
    let attenuation = smoothFactor * smoothFactor;
    return attenuation * 1.0 / max(distanceSquare, 0.0001);
}

// Normal distribution function (specular D)
// Based on https://google.github.io/filament/Filament.html#citation-walter07

// D_GGX(h,α) = α^2 / { π ((n⋅h)^2 (α2−1) + 1)^2 }

// Simple implementation, has precision problems when using fp16 instead of fp32
// see https://google.github.io/filament/Filament.html#listing_speculardfp16
fn D_GGX(roughness: f32, NoH: f32, h: vec3<f32>) -> f32 {
    let oneMinusNoHSquared = 1.0 - NoH * NoH;
    let a = NoH * roughness;
    let k = roughness / (oneMinusNoHSquared + a * a);
    let d = k * k * (1.0 / PI);
    return d;
}

// Visibility function (Specular G)
// V(v,l,a) = G(v,l,α) / { 4 (n⋅v) (n⋅l) }
// such that f_r becomes
// f_r(v,l) = D(h,α) V(v,l,α) F(v,h,f0)
// where
// V(v,l,α) = 0.5 / { n⋅l sqrt((n⋅v)^2 (1−α2) + α2) + n⋅v sqrt((n⋅l)^2 (1−α2) + α2) }
// Note the two sqrt's, that may be slow on mobile, see https://google.github.io/filament/Filament.html#listing_approximatedspecularv
fn V_SmithGGXCorrelated(roughness: f32, NoV: f32, NoL: f32) -> f32 {
    let a2 = roughness * roughness;
    let lambdaV = NoL * sqrt((NoV - a2 * NoV) * NoV + a2);
    let lambdaL = NoV * sqrt((NoL - a2 * NoL) * NoL + a2);
    let v = 0.5 / (lambdaV + lambdaL);
    return v;
}

// Fresnel function
// see https://google.github.io/filament/Filament.html#citation-schlick94
// F_Schlick(v,h,f_0,f_90) = f_0 + (f_90 − f_0) (1 − v⋅h)^5
fn F_Schlick_vec(f0: vec3<f32>, f90: f32, VoH: f32) -> vec3<f32> {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow(1.0 - VoH, 5.0);
}

fn F_Schlick(f0: f32, f90: f32, VoH: f32) -> f32 {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow(1.0 - VoH, 5.0);
}

fn fresnel(f0: vec3<f32>, LoH: f32) -> vec3<f32> {
    // f_90 suitable for ambient occlusion
    // see https://google.github.io/filament/Filament.html#lighting/occlusion
    let f90 = saturate(dot(f0, vec3<f32>(50.0 * 0.33)));
    return F_Schlick_vec(f0, f90, LoH);
}

// Specular BRDF
// https://google.github.io/filament/Filament.html#materialsystem/specularbrdf

// Cook-Torrance approximation of the microfacet model integration using Fresnel law F to model f_m
// f_r(v,l) = { D(h,α) G(v,l,α) F(v,h,f0) } / { 4 (n⋅v) (n⋅l) }
fn specular(f0: vec3<f32>, roughness: f32, h: vec3<f32>, NoV: f32, NoL: f32,
              NoH: f32, LoH: f32, specularIntensity: f32) -> vec3<f32> {
    let D = D_GGX(roughness, NoH, h);
    let V = V_SmithGGXCorrelated(roughness, NoV, NoL);
    let F = fresnel(f0, LoH);

    return (specularIntensity * D * V) * F;
}

// Diffuse BRDF
// https://google.github.io/filament/Filament.html#materialsystem/diffusebrdf
// fd(v,l) = σ/π * 1 / { |n⋅v||n⋅l| } ∫Ω D(m,α) G(v,l,m) (v⋅m) (l⋅m) dm
//
// simplest approximation
// float Fd_Lambert() {
//     return 1.0 / PI;
// }
//
// vec3 Fd = diffuseColor * Fd_Lambert();
//
// Disney approximation
// See https://google.github.io/filament/Filament.html#citation-burley12
// minimal quality difference
fn Fd_Burley(roughness: f32, NoV: f32, NoL: f32, LoH: f32) -> f32 {
    let f90 = 0.5 + 2.0 * roughness * LoH * LoH;
    let lightScatter = F_Schlick(1.0, f90, NoL);
    let viewScatter = F_Schlick(1.0, f90, NoV);
    return lightScatter * viewScatter * (1.0 / PI);
}

// From https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
fn EnvBRDFApprox(f0: vec3<f32>, perceptual_roughness: f32, NoV: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = perceptual_roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * NoV)) * r.x + r.y;
    let AB = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * AB.x + AB.y;
}

fn perceptualRoughnessToRoughness(perceptualRoughness: f32) -> f32 {
    // clamp perceptual roughness to prevent precision problems
    // According to Filament design 0.089 is recommended for mobile
    // Filament uses 0.045 for non-mobile
    let clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089, 1.0);
    return clampedPerceptualRoughness * clampedPerceptualRoughness;
}

// from https://64.github.io/tonemapping/
// reinhard on RGB oversaturates colors
fn reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (1.0 + color);
}

fn reinhard_extended(color: vec3<f32>, max_white: f32) -> vec3<f32> {
    let numerator = color * (1.0f + (color / vec3<f32>(max_white * max_white)));
    return numerator / (1.0 + color);
}

// luminance coefficients from Rec. 709.
// https://en.wikipedia.org/wiki/Rec._709
fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn change_luminance(c_in: vec3<f32>, l_out: f32) -> vec3<f32> {
    let l_in = luminance(c_in);
    return c_in * (l_out / l_in);
}

fn reinhard_luminance(color: vec3<f32>) -> vec3<f32> {
    let l_old = luminance(color);
    let l_new = l_old / (1.0f + l_old);
    return change_luminance(color, l_new);
}

fn reinhard_extended_luminance(color: vec3<f32>, max_white_l: f32) -> vec3<f32> {
    let l_old = luminance(color);
    let numerator = l_old * (1.0f + (l_old / (max_white_l * max_white_l)));
    let l_new = numerator / (1.0f + l_old);
    return change_luminance(color, l_new);
}

fn point_light(
    world_position: vec3<f32>, light: PointLight, roughness: f32, NdotV: f32, N: vec3<f32>, V: vec3<f32>,
    R: vec3<f32>, F0: vec3<f32>, diffuseColor: vec3<f32>
) -> vec3<f32> {
    let light_to_frag = light.position.xyz - world_position.xyz;
    let distance_square = dot(light_to_frag, light_to_frag);
    let rangeAttenuation =
        getDistanceAttenuation(distance_square, light.inverse_square_range);

    // Specular.
    // Representative Point Area Lights.
    // see http://blog.selfshadow.com/publications/s2013-shading-course/karis/s2013_pbs_epic_notes_v2.pdf p14-16
    let a = roughness;
    let centerToRay = dot(light_to_frag, R) * R - light_to_frag;
    let closestPoint = light_to_frag + centerToRay * saturate(light.radius * inverseSqrt(dot(centerToRay, centerToRay)));
    let LspecLengthInverse = inverseSqrt(dot(closestPoint, closestPoint));
    let normalizationFactor = a / saturate(a + (light.radius * 0.5 * LspecLengthInverse));
    let specularIntensity = normalizationFactor * normalizationFactor;

    var L: vec3<f32> = closestPoint * LspecLengthInverse; // normalize() equivalent?
    var H: vec3<f32> = normalize(L + V);
    var NoL: f32 = saturate(dot(N, L));
    var NoH: f32 = saturate(dot(N, H));
    var LoH: f32 = saturate(dot(L, H));

    let specular_light = specular(F0, roughness, H, NdotV, NoL, NoH, LoH, specularIntensity);

    // Diffuse.
    // Comes after specular since its NoL is used in the lighting equation.
    L = normalize(light_to_frag);
    H = normalize(L + V);
    NoL = saturate(dot(N, L));
    NoH = saturate(dot(N, H));
    LoH = saturate(dot(L, H));

    let diffuse = diffuseColor * Fd_Burley(roughness, NdotV, NoL, LoH);

    // Lout = f(v,l) Φ / { 4 π d^2 }⟨n⋅l⟩
    // where
    // f(v,l) = (f_d(v,l) + f_r(v,l)) * light_color
    // Φ is light intensity

    // our rangeAttentuation = 1 / d^2 multiplied with an attenuation factor for smoothing at the edge of the non-physical maximum light radius
    // It's not 100% clear where the 1/4π goes in the derivation, but we follow the filament shader and leave it out

    // See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminanceEquation
    // TODO compensate for energy loss https://google.github.io/filament/Filament.html#materialsystem/improvingthebrdfs/energylossinspecularreflectance
    // light.color.rgb is premultiplied with light.intensity on the CPU

    return ((diffuse + specular_light) * light.color.rgb) * (rangeAttenuation * NoL);
}

fn fetch_shadow(light_id: i32, frag_position: vec4<f32>) -> f32 {
    let light = lights.point_lights[light_id];

    // because the shadow maps align with the axes and the frustum planes are at 45 degrees
    // we can get the worldspace depth by taking the largest absolute axis
    let frag_ls = light.position.xyz - frag_position.xyz;
    let abs_position_ls = abs(frag_ls);
    let major_axis_magnitude = max(abs_position_ls.x, max(abs_position_ls.y, abs_position_ls.z));

    // do a full projection
    // vec4 clip = light.projection * vec4(0.0, 0.0, -major_axis_magnitude, 1.0);
    // float depth = (clip.z / clip.w);

    // alternatively do only the necessary multiplications using near/far
    let proj_r = light.far / (light.near - light.far);
    let z = -major_axis_magnitude * proj_r + light.near * proj_r;
    let w = major_axis_magnitude;
    let depth = z / w;

    // let shadow = texture(samplerCubeArrayShadow(t_Shadow, s_Shadow), vec4(frag_ls, i), depth - bias);

    // manual depth testing
    // float shadow = texture(samplerCubeArray(t_Shadow, s_Shadow), vec4(-frag_ls, 6 * i)).r;
    // shadow = depth > shadow ? 0.0 : 1.0;
    // o_Target = vec4(vec3(shadow * 20 - 19, depth * 20 - 19, 0.0), 1.0);
    // o_Target = vec4(vec3(shadow * 20 - 19), 1.0);

    // do the lookup, using HW PCF and comparison
    // NOTE: Due to the non-uniform control flow above, we must use the Level variant of
    //       textureSampleCompare to avoid undefined behaviour due to some of the fragments in
    //       a quad (2x2 fragments) being processed not being sampled, and this messing with
    //       mip-mapping functionality. The shadow maps have no mipmaps so Level just samples
    //       from LOD 0.
    let bias = 0.0001;
    return textureSampleCompareLevel(shadow_textures, shadow_textures_sampler, frag_ls, i32(light_id), depth - bias);
}

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var output_color: vec4<f32> = material.base_color;
    if ((material.flags & FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }

    // // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((material.flags & FLAGS_UNLIT_BIT) == 0u) {
        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = material.emissive;
        if ((material.flags & FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(emissive_texture, emissive_sampler, in.uv).rgb, 1.0);
        }

        // calculate non-linear roughness from linear perceptualRoughness
        var metallic: f32 = material.metallic;
        var perceptual_roughness: f32 = material.perceptual_roughness;
        if ((material.flags & FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
        let roughness = perceptualRoughnessToRoughness(perceptual_roughness);

        var occlusion: f32 = 1.0;
        if ((material.flags & FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = textureSample(occlusion_texture, occlusion_sampler, in.uv).r;
        }

        var N: vec3<f32> = normalize(in.world_normal);

        // FIXME: Normal maps need an additional vertex attribute and vertex stage output/fragment stage input
        //        Just use a separate shader for lit with normal maps?
        // #    ifdef STANDARDMATERIAL_NORMAL_MAP
        //     vec3 T = normalize(v_WorldTangent.xyz);
        //     vec3 B = cross(N, T) * v_WorldTangent.w;
        // #    endif

        if ((material.flags & FLAGS_DOUBLE_SIDED_BIT) != 0u) {
            if (!in.is_front) {
                N = -N;
            }
        // #        ifdef STANDARDMATERIAL_NORMAL_MAP
        //     T = gl_FrontFacing ? T : -T;
        //     B = gl_FrontFacing ? B : -B;
        // #        endif
        }

        // #    ifdef STANDARDMATERIAL_NORMAL_MAP
        //     mat3 TBN = mat3(T, B, N);
        //     N = TBN * normalize(texture(sampler2D(normal_map, normal_map_sampler), v_Uv).rgb * 2.0 - 1.0);
        // #    endif

        var V: vec3<f32>;
        if (view.view_proj.w.w != 1.0) { // If the projection is not orthographic
            // Only valid for a perpective projection
            V = normalize(view.world_position.xyz - in.world_position.xyz);
        } else {
            // Ortho view vec
            V = normalize(vec3<f32>(-view.view_proj.x.z, -view.view_proj.y.z, -view.view_proj.z.z));
        }

        // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
        let NdotV = max(dot(N, V), 0.0001);

        // Remapping [0,1] reflectance to F0
        // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
        let reflectance = material.reflectance;
        let F0 = 0.16 * reflectance * reflectance * (1.0 - metallic) + output_color.rgb * metallic;

        // Diffuse strength inversely related to metallicity
        let diffuse_color = output_color.rgb * (1.0 - metallic);

        let R = reflect(-V, N);

        // accumulate color
        var light_accum: vec3<f32> = vec3<f32>(0.0);
        for (var i: i32 = 0; i < i32(lights.num_lights); i = i + 1) {
            let light = lights.point_lights[i];
            let light_contrib = point_light(in.world_position.xyz, light, roughness, NdotV, N, V, R, F0, diffuse_color);
            let shadow = fetch_shadow(i, in.world_position);
            light_accum = light_accum + light_contrib * shadow;
        }

        let diffuse_ambient = EnvBRDFApprox(diffuse_color, 1.0, NdotV);
        let specular_ambient = EnvBRDFApprox(F0, perceptual_roughness, NdotV);

        output_color = vec4<f32>(
            light_accum +
                (diffuse_ambient + specular_ambient) * lights.ambient_color.rgb * occlusion +
                emissive.rgb * output_color.a,
            output_color.a);

        // tone_mapping
        output_color = vec4<f32>(reinhard_luminance(output_color.rgb), output_color.a);
        // Gamma correction.
        // Not needed with sRGB buffer
        // output_color.rgb = pow(output_color.rgb, vec3(1.0 / 2.2));
    }

    return output_color;
}
