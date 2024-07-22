/**
 * Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
 * Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
 * Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
 * Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
 * Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
 * of the Software, and to permit persons to whom the Software is furnished to
 * do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software. As clarification, there
 * is no requirement that the copyright notice and permission be included in
 * binary distributions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

/**
 *                  _______  ___  ___       ___           ___
 *                 /       ||   \/   |     /   \         /   \
 *                |   (---- |  \  /  |    /  ^  \       /  ^  \
 *                 \   \    |  |\/|  |   /  /_\  \     /  /_\  \
 *              ----)   |   |  |  |  |  /  _____  \   /  _____  \
 *             |_______/    |__|  |__| /__/     \__\ /__/     \__\
 *
 *                               E N H A N C E D
 *       S U B P I X E L   M O R P H O L O G I C A L   A N T I A L I A S I N G
 *
 *                         http://www.iryoku.com/smaa/
 *
 * Hi, welcome aboard!
 *
 * Here you'll find instructions to get the shader up and running as fast as
 * possible.
 *
 * IMPORTANTE NOTICE: when updating, remember to update both this file and the
 * precomputed textures! They may change from version to version.
 *
 * The shader has three passes, chained together as follows:
 *
 *                           |input|------------------�
 *                              v                     |
 *                    [ SMAA*EdgeDetection ]          |
 *                              v                     |
 *                          |edgesTex|                |
 *                              v                     |
 *              [ SMAABlendingWeightCalculation ]     |
 *                              v                     |
 *                          |blendTex|                |
 *                              v                     |
 *                [ SMAANeighborhoodBlending ] <------�
 *                              v
 *                           |output|
 *
 * Note that each [pass] has its own vertex and pixel shader. Remember to use
 * oversized triangles instead of quads to avoid overshading along the
 * diagonal.
 *
 * You've three edge detection methods to choose from: luma, color or depth.
 * They represent different quality/performance and anti-aliasing/sharpness
 * tradeoffs, so our recommendation is for you to choose the one that best
 * suits your particular scenario:
 *
 * - Depth edge detection is usually the fastest but it may miss some edges.
 *
 * - Luma edge detection is usually more expensive than depth edge detection,
 *   but catches visible edges that depth edge detection can miss.
 *
 * - Color edge detection is usually the most expensive one but catches
 *   chroma-only edges.
 *
 * For quickstarters: just use luma edge detection.
 *
 * The general advice is to not rush the integration process and ensure each
 * step is done correctly (don't try to integrate SMAA T2x with predicated edge
 * detection from the start!). Ok then, let's go!
 *
 *  1. The first step is to create two RGBA temporal render targets for holding
 *     |edgesTex| and |blendTex|.
 *
 *     In DX10 or DX11, you can use a RG render target for the edges texture.
 *     In the case of NVIDIA GPUs, using RG render targets seems to actually be
 *     slower.
 *
 *     On the Xbox 360, you can use the same render target for resolving both
 *     |edgesTex| and |blendTex|, as they aren't needed simultaneously.
 *
 *  2. Both temporal render targets |edgesTex| and |blendTex| must be cleared
 *     each frame. Do not forget to clear the alpha channel!
 *
 *  3. The next step is loading the two supporting precalculated textures,
 *     'areaTex' and 'searchTex'. You'll find them in the 'Textures' folder as
 *     C++ headers, and also as regular DDS files. They'll be needed for the
 *     'SMAABlendingWeightCalculation' pass.
 *
 *     If you use the C++ headers, be sure to load them in the format specified
 *     inside of them.
 *
 *     You can also compress 'areaTex' and 'searchTex' using BC5 and BC4
 *     respectively, if you have that option in your content processor pipeline.
 *     When compressing then, you get a non-perceptible quality decrease, and a
 *     marginal performance increase.
 *
 *  4. All samplers must be set to linear filtering and clamp.
 *
 *     After you get the technique working, remember that 64-bit inputs have
 *     half-rate linear filtering on GCN.
 *
 *     If SMAA is applied to 64-bit color buffers, switching to point filtering
 *     when accessing them will increase the performance. Search for
 *     'SMAASamplePoint' to see which textures may benefit from point
 *     filtering, and where (which is basically the color input in the edge
 *     detection and resolve passes).
 *
 *  5. All texture reads and buffer writes must be non-sRGB, with the exception
 *     of the input read and the output write in
 *     'SMAANeighborhoodBlending' (and only in this pass!). If sRGB reads in
 *     this last pass are not possible, the technique will work anyway, but
 *     will perform antialiasing in gamma space.
 *
 *     IMPORTANT: for best results the input read for the color/luma edge
 *     detection should *NOT* be sRGB.
 *
 *  6. Before including SMAA.h you'll have to setup the render target metrics,
 *     the target and any optional configuration defines. Optionally you can
 *     use a preset.
 *
 *     You have the following targets available:
 *         SMAA_HLSL_3
 *         SMAA_HLSL_4
 *         SMAA_HLSL_4_1
 *         SMAA_GLSL_3 *
 *         SMAA_GLSL_4 *
 *
 *         * (See SMAA_INCLUDE_VS and SMAA_INCLUDE_PS below).
 *
 *     And four presets:
 *         SMAA_PRESET_LOW          (%60 of the quality)
 *         SMAA_PRESET_MEDIUM       (%80 of the quality)
 *         SMAA_PRESET_HIGH         (%95 of the quality)
 *         SMAA_PRESET_ULTRA        (%99 of the quality)
 *
 *     For example:
 *         #define SMAA_RT_METRICS float4(1.0 / 1280.0, 1.0 / 720.0, 1280.0, 720.0)
 *         #define SMAA_HLSL_4
 *         #define SMAA_PRESET_HIGH
 *         #include "SMAA.h"
 *
 *     Note that SMAA_RT_METRICS doesn't need to be a macro, it can be a
 *     uniform variable. The code is designed to minimize the impact of not
 *     using a constant value, but it is still better to hardcode it.
 *
 *     Depending on how you encoded 'areaTex' and 'searchTex', you may have to
 *     add (and customize) the following defines before including SMAA.h:
 *          #define SMAA_AREATEX_SELECT(sample) sample.rg
 *          #define SMAA_SEARCHTEX_SELECT(sample) sample.r
 *
 *     If your engine is already using porting macros, you can define
 *     SMAA_CUSTOM_SL, and define the porting functions by yourself.
 *
 *  7. Then, you'll have to setup the passes as indicated in the scheme above.
 *     You can take a look into SMAA.fx, to see how we did it for our demo.
 *     Checkout the function wrappers, you may want to copy-paste them!
 *
 *  8. It's recommended to validate the produced |edgesTex| and |blendTex|.
 *     You can use a screenshot from your engine to compare the |edgesTex|
 *     and |blendTex| produced inside of the engine with the results obtained
 *     with the reference demo.
 *
 *  9. After you get the last pass to work, it's time to optimize. You'll have
 *     to initialize a stencil buffer in the first pass (discard is already in
 *     the code), then mask execution by using it the second pass. The last
 *     pass should be executed in all pixels.
 *
 *
 * After this point you can choose to enable predicated thresholding,
 * temporal supersampling and motion blur integration:
 *
 * a) If you want to use predicated thresholding, take a look into
 *    SMAA_PREDICATION; you'll need to pass an extra texture in the edge
 *    detection pass.
 *
 * b) If you want to enable temporal supersampling (SMAA T2x):
 *
 * 1. The first step is to render using subpixel jitters. I won't go into
 *    detail, but it's as simple as moving each vertex position in the
 *    vertex shader, you can check how we do it in our DX10 demo.
 *
 * 2. Then, you must setup the temporal resolve. You may want to take a look
 *    into SMAAResolve for resolving 2x modes. After you get it working, you'll
 *    probably see ghosting everywhere. But fear not, you can enable the
 *    CryENGINE temporal reprojection by setting the SMAA_REPROJECTION macro.
 *    Check out SMAA_DECODE_VELOCITY if your velocity buffer is encoded.
 *
 * 3. The next step is to apply SMAA to each subpixel jittered frame, just as
 *    done for 1x.
 *
 * 4. At this point you should already have something usable, but for best
 *    results the proper area textures must be set depending on current jitter.
 *    For this, the parameter 'subsampleIndices' of
 *    'SMAABlendingWeightCalculationPS' must be set as follows, for our T2x
 *    mode:
 *
 *    @SUBSAMPLE_INDICES
 *
 *    | S# |  Camera Jitter   |  subsampleIndices    |
 *    +----+------------------+---------------------+
 *    |  0 |  ( 0.25, -0.25)  |  float4(1, 1, 1, 0)  |
 *    |  1 |  (-0.25,  0.25)  |  float4(2, 2, 2, 0)  |
 *
 *    These jitter positions assume a bottom-to-top y axis. S# stands for the
 *    sample number.
 *
 * More information about temporal supersampling here:
 *    http://iryoku.com/aacourse/downloads/13-Anti-Aliasing-Methods-in-CryENGINE-3.pdf
 *
 * c) If you want to enable spatial multisampling (SMAA S2x):
 *
 * 1. The scene must be rendered using MSAA 2x. The MSAA 2x buffer must be
 *    created with:
 *      - DX10:     see below (*)
 *      - DX10.1:   D3D10_STANDARD_MULTISAMPLE_PATTERN or
 *      - DX11:     D3D11_STANDARD_MULTISAMPLE_PATTERN
 *
 *    This allows to ensure that the subsample order matches the table in
 *    @SUBSAMPLE_INDICES.
 *
 *    (*) In the case of DX10, we refer the reader to:
 *      - SMAA::detectMSAAOrder and
 *      - SMAA::msaaReorder
 *
 *    These functions allow to match the standard multisample patterns by
 *    detecting the subsample order for a specific GPU, and reordering
 *    them appropriately.
 *
 * 2. A shader must be run to output each subsample into a separate buffer
 *    (DX10 is required). You can use SMAASeparate for this purpose, or just do
 *    it in an existing pass (for example, in the tone mapping pass, which has
 *    the advantage of feeding tone mapped subsamples to SMAA, which will yield
 *    better results).
 *
 * 3. The full SMAA 1x pipeline must be run for each separated buffer, storing
 *    the results in the final buffer. The second run should alpha blend with
 *    the existing final buffer using a blending factor of 0.5.
 *    'subsampleIndices' must be adjusted as in the SMAA T2x case (see point
 *    b).
 *
 * d) If you want to enable temporal supersampling on top of SMAA S2x
 *    (which actually is SMAA 4x):
 *
 * 1. SMAA 4x consists on temporally jittering SMAA S2x, so the first step is
 *    to calculate SMAA S2x for current frame. In this case, 'subsampleIndices'
 *    must be set as follows:
 *
 *    | F# | S# |   Camera Jitter    |    Net Jitter     |   subsampleIndices   |
 *    +----+----+--------------------+-------------------+----------------------+
 *    |  0 |  0 |  ( 0.125,  0.125)  |  ( 0.375, -0.125) |  float4(5, 3, 1, 3)  |
 *    |  0 |  1 |  ( 0.125,  0.125)  |  (-0.125,  0.375) |  float4(4, 6, 2, 3)  |
 *    +----+----+--------------------+-------------------+----------------------+
 *    |  1 |  2 |  (-0.125, -0.125)  |  ( 0.125, -0.375) |  float4(3, 5, 1, 4)  |
 *    |  1 |  3 |  (-0.125, -0.125)  |  (-0.375,  0.125) |  float4(6, 4, 2, 4)  |
 *
 *    These jitter positions assume a bottom-to-top y axis. F# stands for the
 *    frame number. S# stands for the sample number.
 *
 * 2. After calculating SMAA S2x for current frame (with the new subsample
 *    indices), previous frame must be reprojected as in SMAA T2x mode (see
 *    point b).
 *
 * e) If motion blur is used, you may want to do the edge detection pass
 *    together with motion blur. This has two advantages:
 *
 * 1. Pixels under heavy motion can be omitted from the edge detection process.
 *    For these pixels we can just store "no edge", as motion blur will take
 *    care of them.
 * 2. The center pixel tap is reused.
 *
 * Note that in this case depth testing should be used instead of stenciling,
 * as we have to write all the pixels in the motion blur pass.
 *
 * That's it!
 */

struct SmaaInfo {
    rt_metrics: vec4<f32>,
}

struct VertexVaryings {
    clip_coord: vec2<f32>,
    tex_coord: vec2<f32>,
}

struct EdgeDetectionVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) offset_0: vec4<f32>,
    @location(1) offset_1: vec4<f32>,
    @location(2) offset_2: vec4<f32>,
    @location(3) tex_coord: vec2<f32>,
}

struct BlendingWeightCalculationVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) offset_0: vec4<f32>,
    @location(1) offset_1: vec4<f32>,
    @location(2) offset_2: vec4<f32>,
    @location(3) tex_coord: vec2<f32>,
}

struct NeighborhoodBlendingVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
}

@group(0) @binding(0) var color_texture: texture_2d<f32>;
@group(0) @binding(1) var<uniform> smaa_info: SmaaInfo;

#ifdef SMAA_EDGE_DETECTION
@group(1) @binding(0) var color_sampler: sampler;
#endif  // SMAA_EDGE_DETECTION

#ifdef SMAA_BLENDING_WEIGHT_CALCULATION
@group(1) @binding(0) var edges_texture: texture_2d<f32>;
@group(1) @binding(1) var edges_sampler: sampler;
@group(1) @binding(2) var search_texture: texture_2d<f32>;
@group(1) @binding(3) var area_texture: texture_2d<f32>;
#endif  // SMAA_BLENDING_WEIGHT_CALCULATION

#ifdef SMAA_NEIGHBORHOOD_BLENDING
@group(1) @binding(0) var blend_texture: texture_2d<f32>;
@group(1) @binding(1) var blend_sampler: sampler;
#endif  // SMAA_NEIGHBORHOOD_BLENDING

//-----------------------------------------------------------------------------
// SMAA Presets

#ifdef SMAA_PRESET_LOW
const SMAA_THRESHOLD: f32 = 0.15;
const SMAA_MAX_SEARCH_STEPS: u32 = 4u;
#define SMAA_DISABLE_DIAG_DETECTION
#define SMAA_DISABLE_CORNER_DETECTION
#else ifdef SMAA_PRESET_MEDIUM  // SMAA_PRESET_LOW
const SMAA_THRESHOLD: f32 = 0.1;
const SMAA_MAX_SEARCH_STEPS: u32 = 8u;
#define SMAA_DISABLE_DIAG_DETECTION
#define SMAA_DISABLE_CORNER_DETECTION
#else ifdef SMAA_PRESET_HIGH    // SMAA_PRESET_MEDIUM
const SMAA_THRESHOLD: f32 = 0.1;
const SMAA_MAX_SEARCH_STEPS: u32 = 16u;
const SMAA_MAX_SEARCH_STEPS_DIAG: u32 = 8u;
const SMAA_CORNER_ROUNDING: u32 = 25u;
#else ifdef SMAA_PRESET_ULTRA   // SMAA_PRESET_HIGH
const SMAA_THRESHOLD: f32 = 0.05;
const SMAA_MAX_SEARCH_STEPS: u32 = 32u;
const SMAA_MAX_SEARCH_STEPS_DIAG: u32 = 16u;
const SMAA_CORNER_ROUNDING: u32 = 25u;
#else                           // SMAA_PRESET_ULTRA
const SMAA_THRESHOLD: f32 = 0.1;
const SMAA_MAX_SEARCH_STEPS: u32 = 16u;
const SMAA_MAX_SEARCH_STEPS_DIAG: u32 = 8u;
const SMAA_CORNER_ROUNDING: u32 = 25u;
#endif                          // SMAA_PRESET_ULTRA

//-----------------------------------------------------------------------------
// Configurable Defines

/**
 * SMAA_THRESHOLD specifies the threshold or sensitivity to edges.
 * Lowering this value you will be able to detect more edges at the expense of
 * performance.
 *
 * Range: [0, 0.5]
 *   0.1 is a reasonable value, and allows to catch most visible edges.
 *   0.05 is a rather overkill value, that allows to catch 'em all.
 *
 *   If temporal supersampling is used, 0.2 could be a reasonable value, as low
 *   contrast edges are properly filtered by just 2x.
 */
// (In the WGSL version of this shader, `SMAA_THRESHOLD` is set above, in "SMAA
// Presets".)

/**
 * SMAA_MAX_SEARCH_STEPS specifies the maximum steps performed in the
 * horizontal/vertical pattern searches, at each side of the pixel.
 *
 * In number of pixels, it's actually the double. So the maximum line length
 * perfectly handled by, for example 16, is 64 (by perfectly, we meant that
 * longer lines won't look as good, but still antialiased).
 *
 * Range: [0, 112]
 */
// (In the WGSL version of this shader, `SMAA_MAX_SEARCH_STEPS` is set above, in
// "SMAA Presets".)

/**
 * SMAA_MAX_SEARCH_STEPS_DIAG specifies the maximum steps performed in the
 * diagonal pattern searches, at each side of the pixel. In this case we jump
 * one pixel at time, instead of two.
 *
 * Range: [0, 20]
 *
 * On high-end machines it is cheap (between a 0.8x and 0.9x slower for 16 
 * steps), but it can have a significant impact on older machines.
 *
 * Define SMAA_DISABLE_DIAG_DETECTION to disable diagonal processing.
 */
// (In the WGSL version of this shader, `SMAA_MAX_SEARCH_STEPS_DIAG` is set
// above, in "SMAA Presets".)

/**
 * SMAA_CORNER_ROUNDING specifies how much sharp corners will be rounded.
 *
 * Range: [0, 100]
 *
 * Define SMAA_DISABLE_CORNER_DETECTION to disable corner processing.
 */
// (In the WGSL version of this shader, `SMAA_CORNER_ROUNDING` is set above, in
// "SMAA Presets".)

/**
 * If there is an neighbor edge that has SMAA_LOCAL_CONTRAST_FACTOR times
 * bigger contrast than current edge, current edge will be discarded.
 *
 * This allows to eliminate spurious crossing edges, and is based on the fact
 * that, if there is too much contrast in a direction, that will hide
 * perceptually contrast in the other neighbors.
 */
const SMAA_LOCAL_CONTRAST_ADAPTATION_FACTOR: f32 = 2.0;

//-----------------------------------------------------------------------------
// Non-Configurable Defines

const SMAA_AREATEX_MAX_DISTANCE: f32 = 16.0;
const SMAA_AREATEX_MAX_DISTANCE_DIAG: f32 = 20.0;
const SMAA_AREATEX_PIXEL_SIZE: vec2<f32> = (1.0 / vec2<f32>(160.0, 560.0));
const SMAA_AREATEX_SUBTEX_SIZE: f32 = (1.0 / 7.0);
const SMAA_SEARCHTEX_SIZE: vec2<f32> = vec2(66.0, 33.0);
const SMAA_SEARCHTEX_PACKED_SIZE: vec2<f32> = vec2(64.0, 16.0);

#ifndef SMAA_DISABLE_CORNER_DETECTION
const SMAA_CORNER_ROUNDING_NORM: f32 = f32(SMAA_CORNER_ROUNDING) / 100.0;
#endif  // SMAA_DISABLE_CORNER_DETECTION

//-----------------------------------------------------------------------------
// WGSL-Specific Functions

// This vertex shader produces the following, when drawn using indices 0..3:
//
//  1 |  0-----x.....2
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  1´
//    +---------------
//      -1  0  1  2  3
//
// The axes are clip-space x and y. The region marked s is the visible region.
// The digits in the corners of the right-angled triangle are the vertex
// indices.
//
// The top-left has UV 0,0, the bottom-left has 0,2, and the top-right has 2,0.
// This means that the UV gets interpolated to 1,1 at the bottom-right corner
// of the clip-space rectangle that is at 1,-1 in clip space.
fn calculate_vertex_varyings(vertex_index: u32) -> VertexVaryings {
    // See the explanation above for how this works
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec2<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0));

    return VertexVaryings(clip_position, uv);
}

//-----------------------------------------------------------------------------
// Vertex Shaders

#ifdef SMAA_EDGE_DETECTION

/**
 * Edge Detection Vertex Shader
 */
@vertex
fn edge_detection_vertex_main(@builtin(vertex_index) vertex_index: u32) -> EdgeDetectionVaryings {
    let varyings = calculate_vertex_varyings(vertex_index);

    var edge_detection_varyings = EdgeDetectionVaryings();
    edge_detection_varyings.position = vec4(varyings.clip_coord, 0.0, 1.0);
    edge_detection_varyings.tex_coord = varyings.tex_coord;

    edge_detection_varyings.offset_0 = smaa_info.rt_metrics.xyxy * vec4(-1.0, 0.0, 0.0, -1.0) +
        varyings.tex_coord.xyxy;
    edge_detection_varyings.offset_1 = smaa_info.rt_metrics.xyxy * vec4(1.0, 0.0, 0.0, 1.0) +
        varyings.tex_coord.xyxy;
    edge_detection_varyings.offset_2 = smaa_info.rt_metrics.xyxy * vec4(-2.0, 0.0, 0.0, -2.0) +
        varyings.tex_coord.xyxy;

    return edge_detection_varyings;
}

#endif  // SMAA_EDGE_DETECTION

#ifdef SMAA_BLENDING_WEIGHT_CALCULATION

/**
 * Blend Weight Calculation Vertex Shader
 */
@vertex
fn blending_weight_calculation_vertex_main(@builtin(vertex_index) vertex_index: u32)
        -> BlendingWeightCalculationVaryings {
    let varyings = calculate_vertex_varyings(vertex_index);

    var weight_varyings = BlendingWeightCalculationVaryings();
    weight_varyings.position = vec4(varyings.clip_coord, 0.0, 1.0);
    weight_varyings.tex_coord = varyings.tex_coord;

    // We will use these offsets for the searches later on (see @PSEUDO_GATHER4):
    weight_varyings.offset_0 = smaa_info.rt_metrics.xyxy * vec4(-0.25, -0.125, 1.25, -0.125) +
        varyings.tex_coord.xyxy;
    weight_varyings.offset_1 = smaa_info.rt_metrics.xyxy * vec4(-0.125, -0.25, -0.125, 1.25) +
        varyings.tex_coord.xyxy;

    // And these for the searches, they indicate the ends of the loops:
    weight_varyings.offset_2 =
        smaa_info.rt_metrics.xxyy * vec4(-2.0, 2.0, -2.0, 2.0) * f32(SMAA_MAX_SEARCH_STEPS) +
        vec4(weight_varyings.offset_0.xz, weight_varyings.offset_1.yw);

    return weight_varyings;
}

#endif  // SMAA_BLENDING_WEIGHT_CALCULATION

#ifdef SMAA_NEIGHBORHOOD_BLENDING

/**
 * Neighborhood Blending Vertex Shader
 */
@vertex
fn neighborhood_blending_vertex_main(@builtin(vertex_index) vertex_index: u32)
        -> NeighborhoodBlendingVaryings {
    let varyings = calculate_vertex_varyings(vertex_index);
    let offset = smaa_info.rt_metrics.xyxy * vec4(1.0, 0.0, 0.0, 1.0) + varyings.tex_coord.xyxy;
    return NeighborhoodBlendingVaryings(
        vec4(varyings.clip_coord, 0.0, 1.0),
        offset,
        varyings.tex_coord
    );
}

#endif  // SMAA_NEIGHBORHOOD_BLENDING

//-----------------------------------------------------------------------------
// Edge Detection Pixel Shaders (First Pass)

#ifdef SMAA_EDGE_DETECTION

/**
 * Luma Edge Detection
 *
 * IMPORTANT NOTICE: luma edge detection requires gamma-corrected colors, and
 * thus 'color_texture' should be a non-sRGB texture.
 */
@fragment
fn luma_edge_detection_fragment_main(in: EdgeDetectionVaryings) -> @location(0) vec4<f32> {
    // Calculate the threshold:
    // TODO: Predication.
    let threshold = vec2(SMAA_THRESHOLD);

    // Calculate luma:
    let weights = vec3(0.2126, 0.7152, 0.0722);
    let L = dot(textureSample(color_texture, color_sampler, in.tex_coord).rgb, weights);

    let Lleft = dot(textureSample(color_texture, color_sampler, in.offset_0.xy).rgb, weights);
    let Ltop  = dot(textureSample(color_texture, color_sampler, in.offset_0.zw).rgb, weights);

    // We do the usual threshold:
    var delta: vec4<f32> = vec4(abs(L - vec2(Lleft, Ltop)), 0.0, 0.0);
    var edges = step(threshold, delta.xy);

    // Then discard if there is no edge:
    if (dot(edges, vec2(1.0)) == 0.0) {
        discard;
    }

    // Calculate right and bottom deltas:
    let Lright  = dot(textureSample(color_texture, color_sampler, in.offset_1.xy).rgb, weights);
    let Lbottom = dot(textureSample(color_texture, color_sampler, in.offset_1.zw).rgb, weights);
    delta = vec4(delta.xy, abs(L - vec2(Lright, Lbottom)));

    // Calculate the maximum delta in the direct neighborhood:
    var max_delta = max(delta.xy, delta.zw);

    // Calculate left-left and top-top deltas:
    let Lleftleft = dot(textureSample(color_texture, color_sampler, in.offset_2.xy).rgb, weights);
    let Ltoptop   = dot(textureSample(color_texture, color_sampler, in.offset_2.zw).rgb, weights);
    delta = vec4(delta.xy, abs(vec2(Lleft, Ltop) - vec2(Lleftleft, Ltoptop)));

    // Calculate the final maximum delta:
    max_delta = max(max_delta.xy, delta.zw);
    let final_delta = max(max_delta.x, max_delta.y);

    // Local contrast adaptation:
    edges *= step(vec2(final_delta), SMAA_LOCAL_CONTRAST_ADAPTATION_FACTOR * delta.xy);

    return vec4(edges, 0.0, 1.0);
}

#endif  // SMAA_EDGE_DETECTION

#ifdef SMAA_BLENDING_WEIGHT_CALCULATION

//-----------------------------------------------------------------------------
// Diagonal Search Functions

#ifndef SMAA_DISABLE_DIAG_DETECTION

/**
 * Allows to decode two binary values from a bilinear-filtered access.
 */
fn decode_diag_bilinear_access_2(in_e: vec2<f32>) -> vec2<f32> {
    // Bilinear access for fetching 'e' have a 0.25 offset, and we are
    // interested in the R and G edges:
    //
    // +---G---+-------+
    // |   x o R   x   |
    // +-------+-------+
    //
    // Then, if one of these edge is enabled:
    //   Red:   (0.75 * X + 0.25 * 1) => 0.25 or 1.0
    //   Green: (0.75 * 1 + 0.25 * X) => 0.75 or 1.0
    //
    // This function will unpack the values (mad + mul + round):
    // wolframalpha.com: round(x * abs(5 * x - 5 * 0.75)) plot 0 to 1
    var e = in_e;
    e.r = e.r * abs(5.0 * e.r - 5.0 * 0.75);
    return round(e);
}

fn decode_diag_bilinear_access_4(e: vec4<f32>) -> vec4<f32> {
    let e_rb = e.rb * abs(5.0 * e.rb - 5.0 * 0.75);
    return round(vec4(e_rb.x, e.g, e_rb.y, e.a));
}

/**
 * These functions allows to perform diagonal pattern searches.
 */
fn search_diag_1(tex_coord: vec2<f32>, dir: vec2<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4(tex_coord, -1.0, 1.0);
    let t = vec3(smaa_info.rt_metrics.xy, 1.0);
    while (coord.z < f32(SMAA_MAX_SEARCH_STEPS_DIAG - 1u) && coord.w > 0.9) {
        coord = vec4(t * vec3(dir, 1.0) + coord.xyz, coord.w);
        *e = textureSampleLevel(edges_texture, edges_sampler, coord.xy, 0.0).rg;
        coord.w = dot(*e, vec2(0.5));
    }
    return coord.zw;
}

fn search_diag_2(tex_coord: vec2<f32>, dir: vec2<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4(tex_coord, -1.0, 1.0);
    coord.x += 0.25 * smaa_info.rt_metrics.x; // See @SearchDiag2Optimization
    let t = vec3(smaa_info.rt_metrics.xy, 1.0);
    while (coord.z < f32(SMAA_MAX_SEARCH_STEPS_DIAG - 1u) && coord.w > 0.9) {
        coord = vec4(t * vec3(dir, 1.0) + coord.xyz, coord.w);

        // @SearchDiag2Optimization
        // Fetch both edges at once using bilinear filtering:
        *e = textureSampleLevel(edges_texture, edges_sampler, coord.xy, 0.0).rg;
        *e = decode_diag_bilinear_access_2(*e);

        // Non-optimized version:
        // e.g = SMAASampleLevelZero(edgesTex, coord.xy).g;
        // e.r = SMAASampleLevelZeroOffset(edgesTex, coord.xy, int2(1, 0)).r;

        coord.w = dot(*e, vec2(0.5));
    }
    return coord.zw;
}

/** 
 * Similar to SMAAArea, this calculates the area corresponding to a certain
 * diagonal distance and crossing edges 'e'.
 */
fn area_diag(dist: vec2<f32>, e: vec2<f32>, offset: f32) -> vec2<f32> {
    var tex_coord = vec2(SMAA_AREATEX_MAX_DISTANCE_DIAG) * e + dist;

    // We do a scale and bias for mapping to texel space:
    tex_coord = SMAA_AREATEX_PIXEL_SIZE * tex_coord + 0.5 * SMAA_AREATEX_PIXEL_SIZE;

    // Diagonal areas are on the second half of the texture:
    tex_coord.x += 0.5;

    // Move to proper place, according to the subpixel offset:
    tex_coord.y += SMAA_AREATEX_SUBTEX_SIZE * offset;

    // Do it!
    return textureSampleLevel(area_texture, edges_sampler, tex_coord, 0.0).rg;
}

/**
 * This searches for diagonal patterns and returns the corresponding weights.
 */
fn calculate_diag_weights(tex_coord: vec2<f32>, e: vec2<f32>, subsample_indices: vec4<f32>)
        -> vec2<f32> {
    var weights = vec2(0.0, 0.0);

    // Search for the line ends:
    var d = vec4(0.0);
    var end = vec2(0.0);
    if (e.r > 0.0) {
        let d_xz = search_diag_1(tex_coord, vec2(-1.0, 1.0), &end);
        d = vec4(d_xz.x, d.y, d_xz.y, d.w);
        d.x += f32(end.y > 0.9);
    } else {
        d = vec4(0.0, d.y, 0.0, d.w);
    }
    let d_yw = search_diag_1(tex_coord, vec2(1.0, -1.0), &end);
    d = vec4(d.x, d_yw.x, d.y, d_yw.y);

    if (d.x + d.y > 2.0) {  // d.x + d.y + 1 > 3
        // Fetch the crossing edges:
        let coords = vec4(-d.x + 0.25, d.x, d.y, -d.y - 0.25) * smaa_info.rt_metrics.xyxy +
            tex_coord.xyxy;
        var c = vec4(
            textureSampleLevel(edges_texture, edges_sampler, coords.xy, 0.0, vec2(-1, 0)).rg,
            textureSampleLevel(edges_texture, edges_sampler, coords.zw, 0.0, vec2( 1, 0)).rg,
        );
        let c_yxwz = decode_diag_bilinear_access_4(c.xyzw);
        c = c_yxwz.yxwz;

        // Non-optimized version:
        // float4 coords = mad(float4(-d.x, d.x, d.y, -d.y), SMAA_RT_METRICS.xyxy, texcoord.xyxy);
        // float4 c;
        // c.x = SMAASampleLevelZeroOffset(edgesTex, coords.xy, int2(-1,  0)).g;
        // c.y = SMAASampleLevelZeroOffset(edgesTex, coords.xy, int2( 0,  0)).r;
        // c.z = SMAASampleLevelZeroOffset(edgesTex, coords.zw, int2( 1,  0)).g;
        // c.w = SMAASampleLevelZeroOffset(edgesTex, coords.zw, int2( 1, -1)).r;

        // Merge crossing edges at each side into a single value:
        var cc = vec2(2.0) * c.xz + c.yw;

        // Remove the crossing edge if we didn't found the end of the line:
        cc = select(cc, vec2(0.0, 0.0), vec2<bool>(step(vec2(0.9), d.zw)));

        // Fetch the areas for this line:
        weights += area_diag(d.xy, cc, subsample_indices.z);
    }

    // Search for the line ends:
    let d_xz = search_diag_2(tex_coord, vec2(-1.0, -1.0), &end);
    if (textureSampleLevel(edges_texture, edges_sampler, tex_coord, 0.0, vec2(1, 0)).r > 0.0) {
        let d_yw = search_diag_2(tex_coord, vec2(1.0, 1.0), &end);
        d = vec4(d_xz.x, d_yw.x, d_xz.y, d_yw.y);
        d.y += f32(end.y > 0.9);
    } else {
        d = vec4(d_xz.x, 0.0, d_xz.y, 0.0);
    }

    if (d.x + d.y > 2.0) {  // d.x + d.y + 1 > 3
        // Fetch the crossing edges:
        let coords = vec4(-d.x, -d.x, d.y, d.y) * smaa_info.rt_metrics.xyxy + tex_coord.xyxy;
        let c = vec4(
            textureSampleLevel(edges_texture, edges_sampler, coords.xy, 0.0, vec2(-1,  0)).g,
            textureSampleLevel(edges_texture, edges_sampler, coords.xy, 0.0, vec2( 0, -1)).r,
            textureSampleLevel(edges_texture, edges_sampler, coords.zw, 0.0, vec2( 1,  0)).gr,
        );
        var cc = vec2(2.0) * c.xz + c.yw;

        // Remove the crossing edge if we didn't found the end of the line:
        cc = select(cc, vec2(0.0, 0.0), vec2<bool>(step(vec2(0.9), d.zw)));

        // Fetch the areas for this line:
        weights += area_diag(d.xy, cc, subsample_indices.w).gr;
    }

    return weights;
}

#endif  // SMAA_DISABLE_DIAG_DETECTION

//-----------------------------------------------------------------------------
// Horizontal/Vertical Search Functions

/**
 * This allows to determine how much length should we add in the last step
 * of the searches. It takes the bilinearly interpolated edge (see 
 * @PSEUDO_GATHER4), and adds 0, 1 or 2, depending on which edges and
 * crossing edges are active.
 */
fn search_length(e: vec2<f32>, offset: f32) -> f32 {
    // The texture is flipped vertically, with left and right cases taking half
    // of the space horizontally:
    var scale = SMAA_SEARCHTEX_SIZE * vec2(0.5, -1.0);
    var bias = SMAA_SEARCHTEX_SIZE * vec2(offset, 1.0);

    // Scale and bias to access texel centers:
    scale += vec2(-1.0,  1.0);
    bias  += vec2( 0.5, -0.5);

    // Convert from pixel coordinates to texcoords:
    // (We use SMAA_SEARCHTEX_PACKED_SIZE because the texture is cropped)
    scale *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;
    bias *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;

    // Lookup the search texture:
    return textureSampleLevel(search_texture, edges_sampler, scale * e + bias, 0.0).r;
}

/**
 * Horizontal/vertical search functions for the 2nd pass.
 */
fn search_x_left(in_tex_coord: vec2<f32>, end: f32) -> f32 {
    var tex_coord = in_tex_coord;

    /**
     * @PSEUDO_GATHER4
     * This texcoord has been offset by (-0.25, -0.125) in the vertex shader to
     * sample between edge, thus fetching four edges in a row.
     * Sampling with different offsets in each direction allows to disambiguate
     * which edges are active from the four fetched ones.
     */
    var e = vec2(0.0, 1.0);
    while (tex_coord.x > end &&
           e.g > 0.8281 &&  // Is there some edge not activated?
           e.r == 0.0) {    // Or is there a crossing edge that breaks the line?
        e = textureSampleLevel(edges_texture, edges_sampler, tex_coord, 0.0).rg;
        tex_coord += -vec2(2.0, 0.0) * smaa_info.rt_metrics.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.0) + 3.25;
    return smaa_info.rt_metrics.x * offset + tex_coord.x;
}

fn search_x_right(in_tex_coord: vec2<f32>, end: f32) -> f32 {
    var tex_coord = in_tex_coord;

    var e = vec2(0.0, 1.0);
    while (tex_coord.x < end &&
           e.g > 0.8281 &&  // Is there some edge not activated?
           e.r == 0.0) {    // Or is there a crossing edge that breaks the line?
        e = textureSampleLevel(edges_texture, edges_sampler, tex_coord, 0.0).rg;
        tex_coord += vec2(2.0, 0.0) * smaa_info.rt_metrics.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e, 0.5) + 3.25;
    return -smaa_info.rt_metrics.x * offset + tex_coord.x;
}

fn search_y_up(in_tex_coord: vec2<f32>, end: f32) -> f32 {
    var tex_coord = in_tex_coord;

    var e = vec2(1.0, 0.0);
    while (tex_coord.y > end &&
           e.r > 0.8281 &&  // Is there some edge not activated?
           e.g == 0.0) {    // Or is there a crossing edge that breaks the line?
        e = textureSampleLevel(edges_texture, edges_sampler, tex_coord, 0.0).rg;
        tex_coord += -vec2(0.0, 2.0) * smaa_info.rt_metrics.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.0) + 3.25;
    return smaa_info.rt_metrics.y * offset + tex_coord.y;
}

fn search_y_down(in_tex_coord: vec2<f32>, end: f32) -> f32 {
    var tex_coord = in_tex_coord;

    var e = vec2(1.0, 0.0);
    while (tex_coord.y < end &&
           e.r > 0.8281 &&  // Is there some edge not activated?
           e.g == 0.0) {    // Or is there a crossing edge that breaks the line?
        e = textureSampleLevel(edges_texture, edges_sampler, tex_coord, 0.0).rg;
        tex_coord += vec2(0.0, 2.0) * smaa_info.rt_metrics.xy;
    }
    let offset = -(255.0 / 127.0) * search_length(e.gr, 0.5) + 3.25;
    return -smaa_info.rt_metrics.y * offset + tex_coord.y;
}

/** 
 * Ok, we have the distance and both crossing edges. So, what are the areas
 * at each side of current edge?
 */
fn area(dist: vec2<f32>, e1: f32, e2: f32, offset: f32) -> vec2<f32> {
    // Rounding prevents precision errors of bilinear filtering:
    var tex_coord = SMAA_AREATEX_MAX_DISTANCE * round(4.0 * vec2(e1, e2)) + dist;

    // We do a scale and bias for mapping to texel space:
    tex_coord = SMAA_AREATEX_PIXEL_SIZE * tex_coord + 0.5 * SMAA_AREATEX_PIXEL_SIZE;

    // Move to proper place, according to the subpixel offset:
    tex_coord.y += SMAA_AREATEX_SUBTEX_SIZE * offset;

    // Do it!
    return textureSample(area_texture, edges_sampler, tex_coord).rg;
}

//-----------------------------------------------------------------------------
// Corner Detection Functions

fn detect_horizontal_corner_pattern(weights: vec2<f32>, tex_coord: vec4<f32>, d: vec2<f32>)
        -> vec2<f32> {
#ifndef SMAA_DISABLE_CORNER_DETECTION
    let left_right = step(d.xy, d.yx);
    var rounding = (1.0 - SMAA_CORNER_ROUNDING_NORM) * left_right;

    rounding /= left_right.x + left_right.y; // Reduce blending for pixels in the center of a line.

    var factor = vec2(1.0, 1.0);
    factor.x -= rounding.x *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.xy, 0.0, vec2(0,  1)).r;
    factor.x -= rounding.y *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.zw, 0.0, vec2(1,  1)).r;
    factor.y -= rounding.x *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.xy, 0.0, vec2(0, -2)).r;
    factor.y -= rounding.y *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.zw, 0.0, vec2(1, -2)).r;

    return weights * saturate(factor);
#else   // SMAA_DISABLE_CORNER_DETECTION
    return weights;
#endif  // SMAA_DISABLE_CORNER_DETECTION
}

fn detect_vertical_corner_pattern(weights: vec2<f32>, tex_coord: vec4<f32>, d: vec2<f32>)
        -> vec2<f32> {
#ifndef SMAA_DISABLE_CORNER_DETECTION
    let left_right = step(d.xy, d.yx);
    var rounding = (1.0 - SMAA_CORNER_ROUNDING_NORM) * left_right;

    rounding /= left_right.x + left_right.y;

    var factor = vec2(1.0, 1.0);
    factor.x -= rounding.x *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.xy, 0.0, vec2( 1, 0)).g;
    factor.x -= rounding.y *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.zw, 0.0, vec2( 1, 1)).g;
    factor.y -= rounding.x *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.xy, 0.0, vec2(-2, 0)).g;
    factor.y -= rounding.y *
        textureSampleLevel(edges_texture, edges_sampler, tex_coord.zw, 0.0, vec2(-2, 1)).g;

    return weights * saturate(factor);
#else   // SMAA_DISABLE_CORNER_DETECTION
    return weights;
#endif  // SMAA_DISABLE_CORNER_DETECTION
}

//-----------------------------------------------------------------------------
// Blending Weight Calculation Pixel Shader (Second Pass)

@fragment
fn blending_weight_calculation_fragment_main(in: BlendingWeightCalculationVaryings)
        -> @location(0) vec4<f32> {
    let subsample_indices = vec4(0.0);  // Just pass zero for SMAA 1x, see @SUBSAMPLE_INDICES.

    var weights = vec4(0.0);

    var e = textureSample(edges_texture, edges_sampler, in.tex_coord).rg;

    if (e.g > 0.0) {    // Edge at north
#ifndef SMAA_DISABLE_DIAG_DETECTION
        // Diagonals have both north and west edges, so searching for them in
        // one of the boundaries is enough.
        weights = vec4(calculate_diag_weights(in.tex_coord, e, subsample_indices), weights.ba);

        // We give priority to diagonals, so if we find a diagonal we skip 
        // horizontal/vertical processing.
        if (weights.r + weights.g != 0.0) {
            return weights;
        }
#endif  // SMAA_DISABLE_DIAG_DETECTION

        var d: vec2<f32>;

        // Find the distance to the left:
        var coords: vec3<f32>;
        coords.x = search_x_left(in.offset_0.xy, in.offset_2.x);
        // in.offset_1.y = in.tex_coord.y - 0.25 * smaa_info.rt_metrics.y (@CROSSING_OFFSET)
        coords.y = in.offset_1.y;
        d.x = coords.x;

        // Now fetch the left crossing edges, two at a time using bilinear
        // filtering. Sampling at -0.25 (see @CROSSING_OFFSET) enables to
        // discern what value each edge has:
        let e1 = textureSampleLevel(edges_texture, edges_sampler, coords.xy, 0.0).r;

        // Find the distance to the right:
        coords.z = search_x_right(in.offset_0.zw, in.offset_2.y);
        d.y = coords.z;

        // We want the distances to be in pixel units (doing this here allow to
        // better interleave arithmetic and memory accesses):
        d = abs(round(smaa_info.rt_metrics.zz * d - in.position.xx));

        // SMAAArea below needs a sqrt, as the areas texture is compressed
        // quadratically:
        let sqrt_d = sqrt(d);

        // Fetch the right crossing edges:
        let e2 = textureSampleLevel(
            edges_texture, edges_sampler, coords.zy, 0.0, vec2<i32>(1, 0)).r;

        // Ok, we know how this pattern looks like, now it is time for getting
        // the actual area:
        weights = vec4(area(sqrt_d, e1, e2, subsample_indices.y), weights.ba);

        // Fix corners:
        coords.y = in.tex_coord.y;
        weights = vec4(
            detect_horizontal_corner_pattern(weights.rg, coords.xyzy, d),
            weights.ba
        );
    }

    if (e.r > 0.0) {    // Edge at west
        var d: vec2<f32>;

        // Find the distance to the top:
        var coords: vec3<f32>;
        coords.y = search_y_up(in.offset_1.xy, in.offset_2.z);
        // in.offset_1.x = in.tex_coord.x - 0.25 * smaa_info.rt_metrics.x
        coords.x = in.offset_0.x;
        d.x = coords.y;

        // Fetch the top crossing edges:
        let e1 = textureSampleLevel(edges_texture, edges_sampler, coords.xy, 0.0).g;

        // Find the distance to the bottom:
        coords.z = search_y_down(in.offset_1.zw, in.offset_2.w);
        d.y = coords.z;

        // We want the distances to be in pixel units:
        d = abs(round(smaa_info.rt_metrics.ww * d - in.position.yy));

        // SMAAArea below needs a sqrt, as the areas texture is compressed
        // quadratically:
        let sqrt_d = sqrt(d);

        // Fetch the bottom crossing edges:
        let e2 = textureSampleLevel(
            edges_texture, edges_sampler, coords.xz, 0.0, vec2<i32>(0, 1)).g;

        // Get the area for this direction:
        weights = vec4(weights.rg, area(sqrt_d, e1, e2, subsample_indices.x));

        // Fix corners:
        coords.x = in.tex_coord.x;
        weights = vec4(weights.rg, detect_vertical_corner_pattern(weights.ba, coords.xyxz, d));
    }

    return weights;
}

#endif  // SMAA_BLENDING_WEIGHT_CALCULATION

#ifdef SMAA_NEIGHBORHOOD_BLENDING

//-----------------------------------------------------------------------------
// Neighborhood Blending Pixel Shader (Third Pass)

@fragment
fn neighborhood_blending_fragment_main(in: NeighborhoodBlendingVaryings) -> @location(0) vec4<f32> {
    // Fetch the blending weights for current pixel:
    let a = vec4(
        textureSample(blend_texture, blend_sampler, in.offset.xy).a,    // Right
        textureSample(blend_texture, blend_sampler, in.offset.zw).g,    // Top
        textureSample(blend_texture, blend_sampler, in.tex_coord).zx,   // Bottom / Left
    );

    // Is there any blending weight with a value greater than 0.0?
    if (dot(a, vec4(1.0)) < 1.0e-5) {
        let color = textureSampleLevel(color_texture, blend_sampler, in.tex_coord, 0.0);
        // TODO: Reprojection
        return color;
    } else {
        let h = max(a.x, a.z) > max(a.y, a.w);  // max(horizontal) > max(vertical)

        // Calculate the blending offsets:
        var blending_offset = vec4(0.0, a.y, 0.0, a.w);
        var blending_weight = a.yw;
        blending_offset = select(blending_offset, vec4(a.x, 0.0, a.z, 0.0), h);
        blending_weight = select(blending_weight, a.xz, h);
        blending_weight /= dot(blending_weight, vec2(1.0));

        // Calculate the texture coordinates:
        let blending_coord =
            blending_offset * vec4(smaa_info.rt_metrics.xy, -smaa_info.rt_metrics.xy) +
            in.tex_coord.xyxy;

        // We exploit bilinear filtering to mix current pixel with the chosen
        // neighbor:
        var color = blending_weight.x *
            textureSampleLevel(color_texture, blend_sampler, blending_coord.xy, 0.0);
        color += blending_weight.y *
            textureSampleLevel(color_texture, blend_sampler, blending_coord.zw, 0.0);

        // TODO: Reprojection

        return color;
    }
}

#endif  // SMAA_NEIGHBORHOOD_BLENDING
