#define_import_path bevy_pbr::lighting

#import bevy_pbr::{
    mesh_view_types::POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE,
    mesh_view_bindings as view_bindings,
    atmosphere::functions::{calculate_visible_sun_ratio, clamp_to_surface},
    atmosphere::bruneton_functions::transmittance_lut_r_mu_to_uv,
}
#import bevy_render::maths::{PI, orthonormalize, solve_cubic}

const LAYER_BASE: u32 = 0;
const LAYER_CLEARCOAT: u32 = 1;

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

// Input to a lighting function for a single layer (either the base layer or the
// clearcoat layer).
struct LayerLightingInput {
    // The normal vector.
    N: vec3<f32>,
    // The reflected vector.
    R: vec3<f32>,
    // The normal vector ⋅ the view vector.
    NdotV: f32,

    // The perceptual roughness of the layer.
    perceptual_roughness: f32,
    // The roughness of the layer.
    roughness: f32,
}

// Input to a lighting function (`point_light`, `spot_light`,
// `directional_light`).
struct LightingInput {
#ifdef STANDARD_MATERIAL_CLEARCOAT
    layers: array<LayerLightingInput, 2>,
#else   // STANDARD_MATERIAL_CLEARCOAT
    layers: array<LayerLightingInput, 1>,
#endif  // STANDARD_MATERIAL_CLEARCOAT

    // The world-space position.
    P: vec3<f32>,
    // The vector to the view.
    V: vec3<f32>,

    // The diffuse color of the material.
    diffuse_color: vec3<f32>,

    // The 0-1 metallic factor of the material.
    metallic: f32,

    // Specular reflectance at the normal incidence angle.
    F0_dielectric: vec3<f32>,
    F0_metallic: vec3<f32>,
    
    // Constants for the BRDF approximation.
    //
    // See `EnvBRDFApprox` in
    // <https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile>.
    // What we call `F_ab` they call `AB`.
    F_ab: vec2<f32>,

#ifdef STANDARD_MATERIAL_CLEARCOAT
    // The strength of the clearcoat layer.
    clearcoat_strength: f32,
#endif  // STANDARD_MATERIAL_CLEARCOAT

#ifdef STANDARD_MATERIAL_ANISOTROPY
    // The anisotropy strength, reflecting the amount of increased roughness in
    // the tangent direction.
    anisotropy: f32,
    // The tangent direction for anisotropy: i.e. the direction in which
    // roughness increases.
    Ta: vec3<f32>,
    // The bitangent direction, which is the cross product of the normal with
    // the tangent direction.
    Ba: vec3<f32>,
#endif  // STANDARD_MATERIAL_ANISOTROPY
}

// Values derived from the `LightingInput` for both diffuse and specular lights.
struct DerivedLightingInput {
    // The half-vector between L, the incident light vector, and V, the view
    // vector.
    H: vec3<f32>,
    // The normal vector ⋅ the incident light vector.
    NdotL: f32,
    // The normal vector ⋅ the half-vector.
    NdotH: f32,
    // The incident light vector ⋅ the half-vector.
    LdotH: f32,
}

// distanceAttenuation is simply the square falloff of light intensity
// combined with a smooth attenuation at the edge of the light radius
//
// light radius is a non-physical construct for efficiency purposes,
// because otherwise every light affects every fragment in the scene
fn getDistanceAttenuation(distanceSquare: f32, inverseRangeSquared: f32) -> f32 {
    return getRangeFalloff(distanceSquare, inverseRangeSquared) * 1.0 / max(distanceSquare, 0.0001);
}

// Falloff without the distance attenuation, for lights that have it baked-in (eg LTC lights)
fn getRangeFalloff(distanceSquare: f32, inverseRangeSquared: f32) -> f32 {
    let factor = distanceSquare * inverseRangeSquared;
    let smoothFactor = saturate(1.0 - factor * factor);
    return smoothFactor * smoothFactor;
}

// Normal distribution function (specular D)
// Based on https://google.github.io/filament/Filament.html#citation-walter07

// D_GGX(h,α) = α^2 / { π ((n⋅h)^2 (α2−1) + 1)^2 }

// Simple implementation, has precision problems when using fp16 instead of fp32
// see https://google.github.io/filament/Filament.html#listing_speculardfp16
fn D_GGX(roughness: f32, NdotH: f32) -> f32 {
    let oneMinusNdotHSquared = 1.0 - NdotH * NdotH;
    let a = NdotH * roughness;
    let k = roughness / (oneMinusNdotHSquared + a * a);
    let d = k * k * (1.0 / PI);
    return d;
}

// An approximation of the anisotropic GGX distribution function.
//
//                                     1
//     D(𝐡) = ───────────────────────────────────────────────────
//            παₜα_b((𝐡 ⋅ 𝐭)² / αₜ²) + (𝐡 ⋅ 𝐛)² / α_b² + (𝐡 ⋅ 𝐧)²)²
//
// * `T` = 𝐭 = the tangent direction = the direction of increased roughness.
//
// * `B` = 𝐛 = the bitangent direction = the direction of decreased roughness.
//
// * `at` = αₜ = the alpha-roughness in the tangent direction.
//
// * `ab` = α_b = the alpha-roughness in the bitangent direction.
//
// This is from the `KHR_materials_anisotropy` spec:
// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md#individual-lights>
fn D_GGX_anisotropic(at: f32, ab: f32, NdotH: f32, TdotH: f32, BdotH: f32) -> f32 {
    let a2 = at * ab;
    let f = vec3(ab * TdotH, at * BdotH, a2 * NdotH);
    let w2 = a2 / dot(f, f);
    let d = a2 * w2 * w2 * (1.0 / PI);
    return d;
}

// Visibility function (Specular G)
// V(v,l,a) = G(v,l,α) / { 4 (n⋅v) (n⋅l) }
// such that f_r becomes
// f_r(v,l) = D(h,α) V(v,l,α) F(v,h,f0)
// where
// V(v,l,α) = 0.5 / { n⋅l sqrt((n⋅v)^2 (1−α2) + α2) + n⋅v sqrt((n⋅l)^2 (1−α2) + α2) }
// Note the two sqrt's, that may be slow on mobile, see https://google.github.io/filament/Filament.html#listing_approximatedspecularv
fn V_SmithGGXCorrelated(roughness: f32, NdotV: f32, NdotL: f32) -> f32 {
    let a2 = roughness * roughness;
    let lambdaV = NdotL * sqrt((NdotV - a2 * NdotV) * NdotV + a2);
    let lambdaL = NdotV * sqrt((NdotL - a2 * NdotL) * NdotL + a2);
    let v = 0.5 / (lambdaV + lambdaL);
    return v;
}

// The visibility function, anisotropic variant.
fn V_GGX_anisotropic(
    at: f32,
    ab: f32,
    NdotL: f32,
    NdotV: f32,
    BdotV: f32,
    TdotV: f32,
    TdotL: f32,
    BdotL: f32,
) -> f32 {
    let GGX_V = NdotL * length(vec3(at * TdotV, ab * BdotV, NdotV));
    let GGX_L = NdotV * length(vec3(at * TdotL, ab * BdotL, NdotL));
    let v = 0.5 / (GGX_V + GGX_L);
    return saturate(v);
}

// Probability-density function that matches the bounded VNDF sampler
// https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf (Listing 2)
fn ggx_vndf_pdf(i: vec3<f32>, NdotH: f32, roughness: f32) -> f32 {
    let ndf = D_GGX(roughness, NdotH);

    // Common terms
    let ai = roughness * i.xy;
    let len2 = dot(ai, ai);
    let t = sqrt(len2 + i.z * i.z);
    if i.z >= 0.0 {
        let a = roughness;
        let s = 1.0 + length(i.xy);
        let a2 = a * a;
        let s2 = s * s;
        let k = (1.0 - a2) * s2 / (s2 + a2 * i.z * i.z);
        return ndf / (2.0 * (k * i.z + t));
    }

    // Backfacing case
    return ndf * (t - i.z) / (2.0 * len2);
}

// https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf (Listing 1)
fn sample_visible_ggx(
    xi: vec2<f32>,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
) -> vec3<f32> {
    let n = normal;
    let alpha = roughness;

    // Decompose view into components parallel/perpendicular to the normal
    let wi_n = dot(view, n);
    let wi_z = -n * wi_n;
    let wi_xy = view + wi_z;

    // Warp view vector to the unit-roughness configuration
    let wi_std = -normalize(alpha * wi_xy + wi_z);

    // Compute wi_std.z once for reuse
    let wi_std_z = dot(wi_std, n);

    // Bounded VNDF sampling
    // Compute the bound parameter k (Eq. 5) and the scaled z–limit b (Eq. 6)
    let s = 1.0 + length(wi_xy);
    let a = clamp(alpha, 0.0, 1.0);
    let a2 = a * a;
    let s2 = s * s;
    let k = (1.0 - a2) * s2 / (s2 + a2 * wi_n * wi_n);
    let b = select(wi_std_z, k * wi_std_z, wi_n > 0.0);

    // Sample a spherical cap in (-b, 1]
    let z = 1.0 - xi.y * (1.0 + b);
    let sin_theta = sqrt(max(0.0, 1.0 - z * z));
    let phi = 2.0 * PI * xi.x - PI;
    let x = sin_theta * cos(phi);
    let y = sin_theta * sin(phi);
    let c_std = vec3f(x, y, z);

    // Rotate the sample so that the normal aligns with +Z
    let TBN = orthonormalize(n);
    let c = TBN * c_std;

    // Half-vector in the standard frame
    let wm_std = c + wi_std;
    let wm_std_z = n * dot(n, wm_std);
    let wm_std_xy = wm_std_z - wm_std;

    // Unwarp back to original roughness and compute microfacet normal
    let H = normalize(alpha * wm_std_xy + wm_std_z);

    // Reflect view to obtain the outgoing (light) direction
    return reflect(-view, H);
}

// Smith geometric shadowing function
fn G_Smith(NdotV: f32, NdotL: f32, roughness: f32) -> f32 {
    let k = roughness / 2.0;
    let GGXL = NdotL / (NdotL * (1.0 - k) + k);
    let GGXV = NdotV / (NdotV * (1.0 - k) + k);
    return GGXL * GGXV;
}

// A simpler, but nonphysical, alternative to Smith-GGX. We use this for
// clearcoat, per the Filament spec.
//
// https://google.github.io/filament/Filament.html#materialsystem/clearcoatmodel#toc4.9.1
fn V_Kelemen(LdotH: f32) -> f32 {
    return 0.25 / (LdotH * LdotH);
}

// Fresnel function
// see https://google.github.io/filament/Filament.html#citation-schlick94
// F_Schlick(v,h,f_0,f_90) = f_0 + (f_90 − f_0) (1 − v⋅h)^5
fn F_Schlick_vec(f0: vec3<f32>, f90: f32, VdotH: f32) -> vec3<f32> {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow(1.0 - VdotH, 5.0);
}

fn F_Schlick(f0: f32, f90: f32, VdotH: f32) -> f32 {
    // not using mix to keep the vec3 and float versions identical
    return f0 + (f90 - f0) * pow(1.0 - VdotH, 5.0);
}

fn fresnel(f0: vec3<f32>, LdotH: f32) -> vec3<f32> {
    // f_90 suitable for ambient occlusion
    // see https://google.github.io/filament/Filament.html#lighting/occlusion
    let f90 = saturate(dot(f0, vec3<f32>(50.0 * 0.33)));
    return F_Schlick_vec(f0, f90, LdotH);
}

// Given distribution, visibility, and Fresnel term, calculates the final
// specular light.
//
// Multiscattering approximation:
// <https://google.github.io/filament/Filament.html#listing_energycompensationimpl>
fn specular_multiscatter(
    D: f32,
    V: f32,
    F: vec3<f32>,
    F0: vec3<f32>,
    F_ab: vec2<f32>,
    specular_intensity: f32,
) -> vec3<f32> {
    var Fr = (specular_intensity * D * V) * F;
    Fr *= 1.0 + F0 * (1.0 / F_ab.x - 1.0);
    return Fr;
}

// Specular BRDF
// https://google.github.io/filament/Filament.html#materialsystem/specularbrdf

// N, V, and L must all be normalized.
fn derive_lighting_input(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>) -> DerivedLightingInput {
    var input: DerivedLightingInput;
    var H: vec3<f32> = normalize(L + V);
    input.H = H;
    input.NdotL = saturate(dot(N, L));
    input.NdotH = saturate(dot(N, H));
    input.LdotH = saturate(dot(L, H));
    return input;
}


// Cook-Torrance approximation of the microfacet model integration using Fresnel law F to model f_m
// f_r(v,l) = { D(h,α) G(v,l,α) F(v,h,f0) } / { 4 (n⋅v) (n⋅l) }
fn specular(
    input: ptr<function, LightingInput>,
    derived_input: ptr<function, DerivedLightingInput>,
    roughness: f32,
    specular_intensity: f32,
) -> vec3<f32> {
    // Unpack.
    let NdotV = (*input).layers[LAYER_BASE].NdotV;
    let F0 = mix((*input).F0_dielectric, (*input).F0_metallic, (*input).metallic);
    let NdotL = (*derived_input).NdotL;
    let NdotH = (*derived_input).NdotH;
    let LdotH = (*derived_input).LdotH;

    // Calculate distribution.
    let D = D_GGX(roughness, NdotH);
    // Calculate visibility.
    let V = V_SmithGGXCorrelated(roughness, NdotV, NdotL);
    // Calculate the Fresnel term.
    let F = fresnel(F0, LdotH);

    // Calculate the specular light.
    let Fr = specular_multiscatter(D, V, F, F0, (*input).F_ab, specular_intensity);
    return Fr;
}

// Calculates the specular light for the clearcoat layer. Returns Fc, the
// Fresnel term, in the first channel, and Frc, the specular clearcoat light, in
// the second channel.
//
// <https://google.github.io/filament/Filament.html#listing_clearcoatbrdf>
fn specular_clearcoat(
    input: ptr<function, LightingInput>,
    derived_input: ptr<function, DerivedLightingInput>,
    clearcoat_strength: f32,
    roughness: f32,
    specular_intensity: f32,
) -> vec2<f32> {
    // Unpack.
    let NdotH = (*derived_input).NdotH;
    let LdotH = (*derived_input).LdotH;

    // Calculate distribution.
    let Dc = D_GGX(roughness, NdotH);
    // Calculate visibility.
    let Vc = V_Kelemen(LdotH);
    // Calculate the Fresnel term.
    let Fc = F_Schlick(0.04, 1.0, LdotH) * clearcoat_strength;
    // Calculate the specular light.
    let Frc = (specular_intensity * Dc * Vc) * Fc;
    return vec2(Fc, Frc);
}

#ifdef STANDARD_MATERIAL_ANISOTROPY

fn specular_anisotropy(
    input: ptr<function, LightingInput>,
    derived_input: ptr<function, DerivedLightingInput>,
    L: vec3<f32>,
    roughness: f32,
    specular_intensity: f32,
) -> vec3<f32> {
    // Unpack.
    let NdotV = (*input).layers[LAYER_BASE].NdotV;
    let V = (*input).V;
    let F0 = mix((*input).F0_dielectric, (*input).F0_metallic, (*input).metallic);
    let anisotropy = (*input).anisotropy;
    let Ta = (*input).Ta;
    let Ba = (*input).Ba;
    let H = (*derived_input).H;
    let NdotL = (*derived_input).NdotL;
    let NdotH = (*derived_input).NdotH;
    let LdotH = (*derived_input).LdotH;

    let TdotL = dot(Ta, L);
    let BdotL = dot(Ba, L);
    let TdotH = dot(Ta, H);
    let BdotH = dot(Ba, H);
    let TdotV = dot(Ta, V);
    let BdotV = dot(Ba, V);

    let ab = roughness * roughness;
    let at = mix(ab, 1.0, anisotropy * anisotropy);

    let Da = D_GGX_anisotropic(at, ab, NdotH, TdotH, BdotH);
    let Va = V_GGX_anisotropic(at, ab, NdotL, NdotV, BdotV, TdotV, TdotL, BdotL);
    let Fa = fresnel(F0, LdotH);

    // Calculate the specular light.
    let Fr = specular_multiscatter(Da, Va, Fa, F0, (*input).F_ab, specular_intensity);
    return Fr;
}

#endif  // STANDARD_MATERIAL_ANISOTROPY

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
fn Fd_Burley(
    input: ptr<function, LightingInput>,
    derived_input: ptr<function, DerivedLightingInput>,
) -> f32 {
    // Unpack.
    let roughness = (*input).layers[LAYER_BASE].roughness;
    let NdotV = (*input).layers[LAYER_BASE].NdotV;
    let NdotL = (*derived_input).NdotL;
    let LdotH = (*derived_input).LdotH;

    let f90 = 0.5 + 2.0 * roughness * LdotH * LdotH;
    let lightScatter = F_Schlick(1.0, f90, NdotL);
    let viewScatter = F_Schlick(1.0, f90, NdotV);
    return lightScatter * viewScatter * (1.0 / PI);
}

// Scale/bias approximation
// https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
// TODO: Use a LUT (more accurate)
fn F_AB(perceptual_roughness: f32, NdotV: f32) -> vec2<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = perceptual_roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * NdotV)) * r.x + r.y;
    // Keep F_ab positive to avoid divide-by-zero in downstream BRDF terms.
    let f_ab_epsilon = 0.00005;
    return max(vec2<f32>(-1.04, 1.04) * a004 + r.zw, vec2<f32>(f_ab_epsilon));
}

fn EnvBRDFApprox(F0: vec3<f32>, F_ab: vec2<f32>) -> vec3<f32> {
    return F0 * F_ab.x + F_ab.y;
}

fn perceptualRoughnessToRoughness(perceptualRoughness: f32) -> f32 {
    // clamp perceptual roughness to prevent precision problems
    // According to Filament design 0.089 is recommended for mobile
    // Filament uses 0.045 for non-mobile
    let clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089, 1.0);
    return clampedPerceptualRoughness * clampedPerceptualRoughness;
}

// this must align with CubemapLayout in decal/clustered.rs
const CUBEMAP_TYPE_CROSS_VERTICAL: u32 = 0;
const CUBEMAP_TYPE_CROSS_HORIZONTAL: u32 = 1;
const CUBEMAP_TYPE_SEQUENCE_VERTICAL: u32 = 2;
const CUBEMAP_TYPE_SEQUENCE_HORIZONTAL: u32 = 3;

const X_PLUS: u32 = 0;
const X_MINUS: u32 = 1;
const Y_PLUS: u32 = 2;
const Y_MINUS: u32 = 3;
const Z_MINUS: u32 = 4;
const Z_PLUS: u32 = 5;

fn cubemap_uv(direction: vec3<f32>, cubemap_type: u32) -> vec2<f32> {
    let abs_direction = abs(direction);
    let max_axis = max(abs_direction.x, max(abs_direction.y, abs_direction.z));

    let face_index = select(
        select(X_PLUS, X_MINUS, direction.x < 0.0),
        select(
            select(Y_PLUS, Y_MINUS, direction.y < 0.0),
            select(Z_PLUS, Z_MINUS, direction.z < 0.0),
            max_axis != abs_direction.y
        ),
        max_axis != abs_direction.x
    );

    var face_uv: vec2<f32>;
    var divisor: f32;
    var corner_uv: vec2<u32> = vec2(0, 0);
    var face_size: vec2<f32>;

    switch face_index {
        case X_PLUS:  { face_uv = vec2<f32>(direction.z, -direction.y); divisor = direction.x; }
        case X_MINUS: { face_uv = vec2<f32>(-direction.z, -direction.y); divisor = -direction.x; }
        case Y_PLUS:  { face_uv = vec2<f32>(direction.x,  -direction.z); divisor = direction.y; }
        case Y_MINUS: { face_uv = vec2<f32>(direction.x, direction.z); divisor = -direction.y; }
        case Z_PLUS:  { face_uv = vec2<f32>(direction.x, direction.y); divisor = direction.z; }
        case Z_MINUS: { face_uv = vec2<f32>(direction.x, -direction.y); divisor = -direction.z; }
        default: {}
    }
    face_uv = (face_uv / divisor) * 0.5 + 0.5;

    switch cubemap_type {
        case CUBEMAP_TYPE_CROSS_VERTICAL: {
            face_size = vec2(1.0/3.0, 1.0/4.0);
            corner_uv = vec2<u32>((0x111102u >> (4 * face_index)) & 0xFu, (0x132011u >> (4 * face_index)) & 0xFu);
        }
        case CUBEMAP_TYPE_CROSS_HORIZONTAL: {
            face_size = vec2(1.0/4.0, 1.0/3.0);
            corner_uv = vec2<u32>((0x131102u >> (4 * face_index)) & 0xFu, (0x112011u >> (4 * face_index)) & 0xFu);
        }
        case CUBEMAP_TYPE_SEQUENCE_HORIZONTAL: {
            face_size = vec2(1.0/6.0, 1.0);
            corner_uv.x = face_index;
        }
        case CUBEMAP_TYPE_SEQUENCE_VERTICAL: {
            face_size = vec2(1.0, 1.0/6.0);
            corner_uv.y = face_index;
        }
        default: {}
    }

    return (vec2<f32>(corner_uv) + face_uv) * face_size;
}

// From the SIGGRAPH 2017 talk, "Real-Time Line- and Disk-Light Shading" by Heitz and Hill
// Slides and code: https://blog.selfshadow.com/publications/s2017-shading-course
fn ltc_integrate_disk(
    N: vec3<f32>,
    V: vec3<f32>,
    P: vec3<f32>,
    Minv: mat3x3<f32>,
    disk_center: vec3<f32>,
    disk_right: vec3<f32>,
    disk_up: vec3<f32>,
) -> f32 {
    // Construct orthonormal basis around N
    let T1 = normalize(V - N * dot(V, N));
    let T2 = cross(N, T1);
    let basis = transpose(mat3x3<f32>(T1, T2, N));

    // Init ellipse
    var C = Minv * (basis * (disk_center - P));
    var V1 = Minv * (basis * disk_right);
    var V2 = Minv * (basis * disk_up);

    if dot(cross(V1, V2), C) < 0.0 {
        return 0.0;
    }

    // Compute eigenvectors of ellipse
    var a: f32;
    var b: f32;
    let d11 = dot(V1, V1);
    let d22 = dot(V2, V2);
    let d12 = dot(V1, V2);

    // V1 and V2 are not orthogonal
    if abs(d12) / sqrt(d11 * d22) > 0.0001 {
        let tr = d11 + d22;
        let det = sqrt(max(d11 * d22 - d12 * d12, 0.0));

        // Use sqrt matrix to solve for eigenvalues
        let u = 0.5 * sqrt(max(tr - 2.0 * det, 0.0));
        let v = 0.5 * sqrt(max(tr + 2.0 * det, 0.0));
        let e_max = (u + v) * (u + v);
        let e_min = (u - v) * (u - v);

        var V1_new: vec3<f32>;
        var V2_new: vec3<f32>;
        if d11 > d22 {
            V1_new = d12 * V1 + (e_max - d11) * V2;
            V2_new = d12 * V1 + (e_min - d11) * V2;
        } else {
            V1_new = d12 * V2 + (e_max - d22) * V1;
            V2_new = d12 * V2 + (e_min - d22) * V1;
        }

        a = 1.0 / e_max;
        b = 1.0 / e_min;
        V1 = normalize(V1_new);
        V2 = normalize(V2_new);
    } else {
        a = 1.0 / d11;
        b = 1.0 / d22;
        V1 = V1 * sqrt(a);
        V2 = V2 * sqrt(b);
    }

    var V3 = cross(V1, V2);
    if dot(C, V3) < 0.0 {
        V3 = -V3;
    }

    let L = dot(V3, C);
    let x0 = dot(V1, C) / L;
    let y0 = dot(V2, C) / L;

    let aL2 = a * L * L;
    let bL2 = b * L * L;

    let c0 = aL2 * bL2;
    let c1 = aL2 * bL2 * (1.0 + x0 * x0 + y0 * y0) - aL2 - bL2;
    let c2 = 1.0 - aL2 * (1.0 + x0 * x0) - bL2 * (1.0 + y0 * y0);
    let c3 = 1.0;

    let roots = solve_cubic(vec4(c0, c1, c2, c3));
    let e1 = roots.x;
    let e2 = roots.y;
    let e3 = roots.z;

    let avg_dir_local = vec3(aL2 * x0 / (aL2 - e2), bL2 * y0 / (bL2 - e2), 1.0);
    let rotate = mat3x3<f32>(V1, V2, V3);
    let avg_dir = normalize(rotate * avg_dir_local);

    let L1 = sqrt(max(-e2 / e3, 0.0));
    let L2 = sqrt(max(-e2 / e1, 0.0));

    let form_factor = L1 * L2 * inverseSqrt((1.0 + L1 * L1) * (1.0 + L2 * L2));

    // Use tabulated horizon-clipped sphere
    let LUT_SCALE = 63.0 / 64.0;
    let LUT_BIAS =  0.5 / 64.0;
    let clip_uv = vec2(avg_dir.z * 0.5 + 0.5, form_factor) * LUT_SCALE + LUT_BIAS;
    let horizon_scale = textureSampleLevel(view_bindings::ltc_lut2, view_bindings::ltc_lut2_sampler, clip_uv, 0.0).w;

    return form_factor * horizon_scale;
}

fn point_light(
    light_id: u32,
    input: ptr<function, LightingInput>,
    enable_diffuse: bool,
    enable_texture: bool,
) -> vec3<f32> {
    // Unpack.
    let diffuse_color = (*input).diffuse_color;
    let P = (*input).P;
    let N = (*input).layers[LAYER_BASE].N;
    let V = (*input).V;

    let light = &view_bindings::clustered_lights.data[light_id];
    let light_to_frag = (*light).position_radius.xyz - P;
    let L = normalize(light_to_frag);
    let distance_square = dot(light_to_frag, light_to_frag);
    let distance = sqrt(distance_square);
    let light_radius = (*light).position_radius.w;
    let inverse_range_squared = (*light).color_inverse_square_range.w;
    let range_falloff = getRangeFalloff(distance_square, inverse_range_squared);
    let range_attenuation = getDistanceAttenuation(distance_square, inverse_range_squared);

    var color_times_NdotL: vec3<f32>;

    if light_radius == 0.0 {
        // Base layer
        let a = (*input).layers[LAYER_BASE].roughness;
        let specular_intensity = 1.0;
        var specular_derived_input = derive_lighting_input(N, V, L);

#ifdef STANDARD_MATERIAL_ANISOTROPY
        var specular_light = specular_anisotropy(input, &specular_derived_input, L, a, specular_intensity);
#else   // STANDARD_MATERIAL_ANISOTROPY
        var specular_light = specular(input, &specular_derived_input, a, specular_intensity);
#endif  // STANDARD_MATERIAL_ANISOTROPY

        // Clearcoat
#ifdef STANDARD_MATERIAL_CLEARCOAT
        // Unpack.
        let clearcoat_N = (*input).layers[LAYER_CLEARCOAT].N;
        let clearcoat_strength = (*input).clearcoat_strength;
        let clearcoat_a = (*input).layers[LAYER_CLEARCOAT].roughness;

        // Perform specular input calculations again for the clearcoat layer. We
        // can't reuse the above because the clearcoat normal might be different
        // from the main layer normal.
        var clearcoat_specular_derived_input = derive_lighting_input(clearcoat_N, V, L);
        let Fc_Frc = specular_clearcoat(input, &clearcoat_specular_derived_input, clearcoat_strength, clearcoat_a, 1.0);
        let inv_Fc = 1.0 - Fc_Frc.r;    // Inverse Fresnel term.
        var Frc = Fc_Frc.g;             // Clearcoat light.
#endif  // STANDARD_MATERIAL_CLEARCOAT

        // Diffuse.
        // Comes after specular since its N⋅L is used in the lighting equation.
        var derived_input = derive_lighting_input(N, V, L);
        var diffuse = vec3(0.0);
        if (enable_diffuse) {
            diffuse = diffuse_color * Fd_Burley(input, &derived_input);
        }

        // See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminanceEquation
        // Lout = f(v,l) Φ / { 4 π d^2 }⟨n⋅l⟩
        // where
        // f(v,l) = (f_d(v,l) + f_r(v,l)) * light_color
        // Φ is luminous power in lumens
        // our rangeAttenuation = 1 / d^2 multiplied with an attenuation factor for smoothing at the edge of the non-physical maximum light radius

        // For a point light, luminous intensity, I, in lumens per steradian is given by:
        // I = Φ / 4 π
        // The derivation of this can be seen here: https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower

        // NOTE: (*light).color.rgb is premultiplied with (*light).intensity / 4 π (which would be the luminous intensity) on the CPU
#ifdef STANDARD_MATERIAL_CLEARCOAT
        // Account for the Fresnel term from the clearcoat darkening the main layer.
        //
        // <https://google.github.io/filament/Filament.html#materialsystem/clearcoatmodel/integrationinthesurfaceresponse>
        color_times_NdotL = (diffuse * derived_input.NdotL + specular_light * specular_derived_input.NdotL * inv_Fc) * inv_Fc + Frc * clearcoat_specular_derived_input.NdotL;
#else   // STANDARD_MATERIAL_CLEARCOAT
        color_times_NdotL = diffuse * derived_input.NdotL + specular_light * specular_derived_input.NdotL;
#endif  // STANDARD_MATERIAL_CLEARCOAT
    } else { 
        // light_radius > 0, use LTC

        let basis = orthonormalize(L);
        let disk_right = basis[0] * light_radius;
        let disk_up = basis[1] * light_radius;

        let NdotV = (*input).layers[LAYER_BASE].NdotV;
        let perceptual_roughness = (*input).layers[LAYER_BASE].perceptual_roughness;

        // Sample the LTC LUTs 
        let LUT_SCALE = 63.0 / 64.0;
        let LUT_BIAS  =  0.5 / 64.0;
        let uv = vec2(perceptual_roughness, sqrt(1.0 - NdotV)) * LUT_SCALE + LUT_BIAS;
        let t1 = textureSampleLevel(view_bindings::ltc_lut1, view_bindings::ltc_lut1_sampler, uv, 0.0);
        let t2 = textureSampleLevel(view_bindings::ltc_lut2, view_bindings::ltc_lut2_sampler, uv, 0.0);

        // Reconstruct the GGX inverse-LTC matrix
        let Minv = mat3x3<f32>(
            vec3(t1.x, 0.0, t1.y),
            vec3(0.0,  1.0, 0.0),
            vec3(t1.z, 0.0, t1.w),
        );
        let spec = ltc_integrate_disk(N, V, P, Minv, (*light).position_radius.xyz, disk_right, disk_up);

        // Use Lambertian diffuse, Burley would require a second LUT
        let identity = mat3x3<f32>(
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        );
        let diff = select(0.0, ltc_integrate_disk(N, V, P, identity, (*light).position_radius.xyz, disk_right, disk_up), enable_diffuse);

        // t2.x = BSDF magnitude, t2.y = Fresnel weight
        let F0 = mix((*input).F0_dielectric, (*input).F0_metallic, (*input).metallic);
        let spec_weight = F0 * t2.x + (1.0 - F0) * t2.y;

#ifdef STANDARD_MATERIAL_CLEARCOAT
        let clearcoat_N = (*input).layers[LAYER_CLEARCOAT].N;
        let clearcoat_strength = (*input).clearcoat_strength;
        let clearcoat_perceptual_roughness = (*input).layers[LAYER_CLEARCOAT].perceptual_roughness;
        let clearcoat_NdotV = (*input).layers[LAYER_CLEARCOAT].NdotV;

        // Sample LUTs for clearcoat layer
        let cc_uv = vec2<f32>(clearcoat_perceptual_roughness, sqrt(1.0 - clearcoat_NdotV)) * LUT_SCALE + LUT_BIAS;
        let tc1 = textureSampleLevel(view_bindings::ltc_lut1, view_bindings::ltc_lut1_sampler, cc_uv, 0.0);
        let tc2 = textureSampleLevel(view_bindings::ltc_lut2, view_bindings::ltc_lut2_sampler, cc_uv, 0.0);
        let Minv_cc = mat3x3<f32>(
            vec3<f32>(tc1.x, 0.0, tc1.y),
            vec3<f32>(0.0,   1.0, 0.0),
            vec3<f32>(tc1.z, 0.0, tc1.w),
        );
        let spec_cc = ltc_integrate_disk(clearcoat_N, V, P, Minv_cc, (*light).position_radius.xyz, disk_right, disk_up);

        // Clearcoat has F0=0.04
        let spec_weight_cc = 0.04 * tc2.x + (1.0 - 0.04) * tc2.y;
        let Fc = clearcoat_strength * spec_weight_cc;
        let inv_Fc = 1.0 - Fc;
        color_times_NdotL = (spec_weight * spec * inv_Fc + diffuse_color * diff) * inv_Fc
            + spec_weight_cc * spec_cc * clearcoat_strength;
#else   // STANDARD_MATERIAL_CLEARCOAT
        color_times_NdotL = spec_weight * spec + diffuse_color * diff;
#endif  // STANDARD_MATERIAL_CLEARCOAT

    // LTC integration expects intensity in cd/m²
    color_times_NdotL *= 1.0 / max(PI * light_radius * light_radius, 1e-7);
    }

    var texture_sample = 1f;
#ifdef LIGHT_TEXTURES
    if enable_texture && (*light).decal_index != 0xFFFFFFFFu {
        let relative_position = (view_bindings::clustered_decals.decals[(*light).decal_index].local_from_world * vec4(P, 1.0)).xyz;
        let cubemap_type = view_bindings::clustered_decals.decals[(*light).decal_index].tag;
        let decal_uv = cubemap_uv(relative_position, cubemap_type);
        let image_index = view_bindings::clustered_decals.decals[(*light).decal_index].base_color_texture_index;

        texture_sample = textureSampleLevel(
            view_bindings::clustered_decal_textures[image_index],
            view_bindings::clustered_decal_sampler,
            decal_uv,
            0.0
        ).r;
    }
#endif

    // Punctual path (radius == 0) uses inverse-square falloff.
    // LTC path (radius > 0) uses only the smooth range cutoff to avoid
    // double-applying distance attenuation.
    return color_times_NdotL
        * (*light).color_inverse_square_range.rgb
        * texture_sample
        * select(range_attenuation, range_falloff, light_radius > 0.0);
}

fn spot_light(
    light_id: u32,
    input: ptr<function, LightingInput>,
    enable_diffuse: bool
) -> vec3<f32> {
    // reuse the point light calculations
    let point_light = point_light(light_id, input, enable_diffuse, false);

    let light = &view_bindings::clustered_lights.data[light_id];

    // reconstruct spot dir from x/z and y-direction flag
    var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
    spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
    if ((*light).flags & POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u {
        spot_dir.y = -spot_dir.y;
    }
    let light_to_frag = (*light).position_radius.xyz - (*input).P.xyz;

    // calculate attenuation based on filament formula https://google.github.io/filament/Filament.html#listing_glslpunctuallight
    // spot_scale and spot_offset have been precomputed
    // note we normalize here to get "l" from the filament listing. spot_dir is already normalized
    let cd = dot(-spot_dir, normalize(light_to_frag));
    let attenuation = saturate(cd * (*light).light_custom_data.z + (*light).light_custom_data.w);
    let spot_attenuation = attenuation * attenuation;

    var texture_sample = 1f;

#ifdef LIGHT_TEXTURES
    if (*light).decal_index != 0xFFFFFFFFu {
        let local_position = (view_bindings::clustered_decals.decals[(*light).decal_index].local_from_world *
            vec4((*input).P, 1.0)).xyz;
        if local_position.z < 0.0 {
            let decal_uv = (local_position.xy / (local_position.z * (*light).spot_light_tan_angle)) * vec2(-0.5, 0.5) + 0.5;
            let image_index = view_bindings::clustered_decals.decals[(*light).decal_index].base_color_texture_index;

            texture_sample = textureSampleLevel(
                view_bindings::clustered_decal_textures[image_index],
                view_bindings::clustered_decal_sampler,
                decal_uv,
                0.0
            ).r;
        }
    }
#endif

    return point_light * spot_attenuation * texture_sample;
}

fn directional_light(
    light_id: u32,
    input: ptr<function, LightingInput>,
    enable_diffuse: bool
) -> vec3<f32> {
    // Unpack.
    let diffuse_color = (*input).diffuse_color;
    let NdotV = (*input).layers[LAYER_BASE].NdotV;
    let N = (*input).layers[LAYER_BASE].N;
    let V = (*input).V;
    let roughness = (*input).layers[LAYER_BASE].roughness;

    let light = &view_bindings::lights.directional_lights[light_id];

    let L = (*light).direction_to_light.xyz;
    var derived_input = derive_lighting_input(N, V, L);

    var diffuse = vec3(0.0);
    if (enable_diffuse) {
        diffuse = diffuse_color * Fd_Burley(input, &derived_input);
    }

#ifdef STANDARD_MATERIAL_ANISOTROPY
    let specular_light = specular_anisotropy(input, &derived_input, L, roughness, 1.0);
#else   // STANDARD_MATERIAL_ANISOTROPY
    let specular_light = specular(input, &derived_input, roughness, 1.0);
#endif  // STANDARD_MATERIAL_ANISOTROPY

#ifdef STANDARD_MATERIAL_CLEARCOAT
    let clearcoat_N = (*input).layers[LAYER_CLEARCOAT].N;
    let clearcoat_strength = (*input).clearcoat_strength;
    let clearcoat_roughness = (*input).layers[LAYER_CLEARCOAT].roughness;

    // Perform specular input calculations again for the clearcoat layer. We
    // can't reuse the above because the clearcoat normal might be different
    // from the main layer normal.
    var derived_clearcoat_input = derive_lighting_input(clearcoat_N, V, L);

    let Fc_Frc =
        specular_clearcoat(input, &derived_clearcoat_input, clearcoat_strength, clearcoat_roughness, 1.0);
    let inv_Fc = 1.0 - Fc_Frc.r;
    let Frc = Fc_Frc.g;
#endif  // STANDARD_MATERIAL_CLEARCOAT

    var color: vec3<f32>;
#ifdef STANDARD_MATERIAL_CLEARCOAT
    // Account for the Fresnel term from the clearcoat darkening the main layer.
    //
    // <https://google.github.io/filament/Filament.html#materialsystem/clearcoatmodel/integrationinthesurfaceresponse>
    color = (diffuse + specular_light * inv_Fc) * inv_Fc * derived_input.NdotL +
        Frc * derived_clearcoat_input.NdotL;
#else   // STANDARD_MATERIAL_CLEARCOAT
    color = (diffuse + specular_light) * derived_input.NdotL;
#endif  // STANDARD_MATERIAL_CLEARCOAT

    var texture_sample = 1f;

#ifdef LIGHT_TEXTURES
    if (*light).decal_index != 0xFFFFFFFFu {
        let local_position = (view_bindings::clustered_decals.decals[(*light).decal_index].local_from_world *
            vec4((*input).P, 1.0)).xyz;
        let decal_uv = local_position.xy * vec2(-0.5, 0.5) + 0.5;

        // if tiled or within tile
        if (view_bindings::clustered_decals.decals[(*light).decal_index].tag != 0u)
                || all(clamp(decal_uv, vec2(0.0), vec2(1.0)) == decal_uv)
        {
            let image_index = view_bindings::clustered_decals.decals[(*light).decal_index].base_color_texture_index;

            texture_sample = textureSampleLevel(
                view_bindings::clustered_decal_textures[image_index],
                view_bindings::clustered_decal_sampler,
                decal_uv - floor(decal_uv),
                0.0
            ).r;
        } else {
            texture_sample = 0f;
        }
    }
#endif

color *= (*light).color.rgb * texture_sample;

#ifdef ATMOSPHERE
    let P = (*input).P;
    let atmosphere = view_bindings::atmosphere_data.atmosphere;
    let O = vec3(0.0, atmosphere.bottom_radius, 0.0);
    let P_scaled = P * vec3(view_bindings::atmosphere_data.settings.scene_units_to_m);
    let P_as = P_scaled + O;
    let P_clamped = clamp_to_surface(atmosphere, P_as);
    let r = length(P_clamped);
    let local_up = normalize(P_clamped);
    let mu_light = dot(L, local_up);

    // Sample atmosphere
    let transmittance = sample_transmittance_lut(r, mu_light);
    let sun_visibility = calculate_visible_sun_ratio(atmosphere, r, mu_light, (*light).sun_disk_angular_size);
    
    // Apply atmospheric effects
    color *= transmittance * sun_visibility;
#endif

    return color;
}

// Linearly Transformed Cosines (LTC) area light evaluation.
// Based on: Heitz et al. 2016, "Real-Time Polygonal-Light Shading with Linearly Transformed Cosines"

// Integrate one edge of the spherical polygon formed by the LTC-transformed quad.
// Implements Eq. 11 using a polynomial approximation based on https://advances.realtimerendering.com/s2016/s2016_ltc_rnd.pdf
// Using the equation as-is produces visible artifacts.
fn ltc_integrate_edge(v1: vec3<f32>, v2: vec3<f32>) -> f32 {
    let x = dot(v1, v2);
    let y = abs(x);
    let a = 0.8543985 + (0.4965155 + 0.0145206 * y) * y;
    let b = 3.4175940 + (4.1616724 + y) * y;
    let v = a / b;
    let theta_sintheta = select(0.5 * inverseSqrt(max(1.0 - x * x, 1e-7)) - v, v, x > 0.0);
    return cross(v1, v2).z * theta_sintheta;
}

fn ltc_integrate_quad(
    N: vec3<f32>,
    V: vec3<f32>,
    P: vec3<f32>,
    Minv: mat3x3<f32>,
    points: array<vec3<f32>, 4>
) -> f32 {
    let T1 = normalize(V - N * dot(V, N));
    let T2 = -cross(N, T1);

    let Minv_local = Minv * transpose(mat3x3<f32>(T1, T2, N));

    // Transform quad vertices into LTC-distorted space
    var L: array<vec3<f32>, 4>;
    L[0] = Minv_local * (points[0] - P);
    L[1] = Minv_local * (points[1] - P);
    L[2] = Minv_local * (points[2] - P);
    L[3] = Minv_local * (points[3] - P);

    // Clip against z >= 0 hemisphere (at most 5 verts output for a quad)

    // TODO: clipping could be made cheaper with a spherical proxy, see https://advances.realtimerendering.com/s2016/s2016_ltc_rnd.pdf slides 87-102
    var clipped: array<vec3<f32>, 5>;
    var n_clipped: i32 = 0;

    for (var i = 0i; i < 4i; i++) {
        let a = L[i];
        let b = L[(i + 1) % 4];
        if (a.z >= 0.0) {
            clipped[n_clipped] = a; 
            n_clipped++;
        }
        if ((a.z >= 0.0) != (b.z >= 0.0)) {
            let t = a.z / (a.z - b.z);
            clipped[n_clipped] = mix(a, b, t);  
            n_clipped++;
        }
    }

    if (n_clipped == 0) {
        return 0.0;
    }

    for (var i = 0i; i < n_clipped; i++) {
        clipped[i] = normalize(clipped[i]);
    }

    // Sum edge integrals over clipped polygon
    var sum = 0.0;
    for (var i = 0i; i < n_clipped; i++) {
        sum += ltc_integrate_edge(clipped[i], clipped[(i + 1) % n_clipped]);
    }

    return sum;
}

fn rect_light(
    light_id: u32,
    input: ptr<function, LightingInput>,
    enable_diffuse: bool,
) -> vec3<f32> {
    // Unpack
    let diffuse_color = (*input).diffuse_color;
    let P = (*input).P;
    let N = (*input).layers[LAYER_BASE].N;
    let V = (*input).V;
    let NdotV = (*input).layers[LAYER_BASE].NdotV;
    let perceptual_roughness = (*input).layers[LAYER_BASE].perceptual_roughness;

    let light = &view_bindings::lights.rect_lights[light_id];
    let light_to_frag = (*light).position - P;
    let distance_square = dot(light_to_frag, light_to_frag);
    let inverse_range_squared = 1.0 / max((*light).range * (*light).range, 0.0001);
    let range_falloff = getRangeFalloff(distance_square, inverse_range_squared);

    let light_normal = cross((*light).up, (*light).right);
    let hw = (*light).right * (*light).width  * 0.5;
    let hh = (*light).up   * (*light).height * 0.5;
    var corners: array<vec3<f32>, 4>;
    corners[0] = (*light).position + hw - hh;
    corners[1] = (*light).position - hw - hh;
    corners[2] = (*light).position - hw + hh;
    corners[3] = (*light).position + hw + hh;

    // Backface test
    if dot(light_normal, P - corners[0]) <= 0.0 {
        return vec3<f32>(0.0);
    }

    let LUT_SCALE = 63.0 / 64.0;
    let LUT_BIAS  =  0.5 / 64.0;
    let uv = vec2<f32>(perceptual_roughness, sqrt(1.0 - NdotV)) * LUT_SCALE + LUT_BIAS;
    let t1 = textureSampleLevel(view_bindings::ltc_lut1, view_bindings::ltc_lut1_sampler, uv, 0.0);
    let t2 = textureSampleLevel(view_bindings::ltc_lut2, view_bindings::ltc_lut2_sampler, uv, 0.0);

    // Reconstruct the GGX inverse-LTC matrix
    let Minv = mat3x3<f32>(
        vec3<f32>(t1.x, 0.0, t1.y),
        vec3<f32>(0.0,  1.0, 0.0),
        vec3<f32>(t1.z, 0.0, t1.w),
    );
    let spec = ltc_integrate_quad(N, V, P, Minv, corners);

    // Use Lambertian diffuse, Burley would require a second LUT
    let identity = mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );
    let diff = select(0.0, ltc_integrate_quad(N, V, P, identity, corners), enable_diffuse);

    // t2.x encodes the bsdf magnitude and t2.y the fresnel direction
    let F0 = mix((*input).F0_dielectric, (*input).F0_metallic, (*input).metallic);
    let spec_weight = F0 * t2.x + (1.0 - F0) * t2.y;

#ifdef STANDARD_MATERIAL_CLEARCOAT
    let clearcoat_N = (*input).layers[LAYER_CLEARCOAT].N;
    let clearcoat_strength = (*input).clearcoat_strength;
    let clearcoat_perceptual_roughness = (*input).layers[LAYER_CLEARCOAT].perceptual_roughness;
    let clearcoat_NdotV = (*input).layers[LAYER_CLEARCOAT].NdotV;

    // Sample LUTs for clearcoat layer
    let cc_uv = vec2<f32>(clearcoat_perceptual_roughness, sqrt(1.0 - clearcoat_NdotV)) * LUT_SCALE + LUT_BIAS;
    let tc1 = textureSampleLevel(view_bindings::ltc_lut1, view_bindings::ltc_lut1_sampler, cc_uv, 0.0);
    let tc2 = textureSampleLevel(view_bindings::ltc_lut2, view_bindings::ltc_lut2_sampler, cc_uv, 0.0);
    let Minv_cc = mat3x3<f32>(
        vec3<f32>(tc1.x, 0.0, tc1.y),
        vec3<f32>(0.0,   1.0, 0.0),
        vec3<f32>(tc1.z, 0.0, tc1.w),
    );
    let spec_cc = ltc_integrate_quad(clearcoat_N, V, P, Minv_cc, corners);

    // Clearcoat has F0=0.04
    let spec_weight_cc = 0.04 * tc2.x + (1.0 - 0.04) * tc2.y;
    let Fc = clearcoat_strength * spec_weight_cc;
    let inv_Fc = 1.0 - Fc;

    return ((spec_weight * spec * inv_Fc + diffuse_color * diff) * inv_Fc
        + spec_weight_cc * spec_cc * clearcoat_strength) * (*light).color.rgb * range_falloff;
#else
    return (spec_weight * spec + diffuse_color * diff) * (*light).color.rgb * range_falloff;
#endif
}


#ifdef ATMOSPHERE
fn sample_transmittance_lut(r: f32, mu: f32) -> vec3<f32> {
    let uv = transmittance_lut_r_mu_to_uv(view_bindings::atmosphere_data.atmosphere, r, mu);
    return textureSampleLevel(
        view_bindings::atmosphere_transmittance_texture, 
        view_bindings::atmosphere_transmittance_sampler, uv, 0.0).rgb;
}
#endif  // ATMOSPHERE
