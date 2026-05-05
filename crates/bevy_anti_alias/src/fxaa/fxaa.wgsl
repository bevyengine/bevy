// NVIDIA FXAA 3.11
// Original source code by TIMOTHY LOTTES
// https://gist.github.com/kosua20/0c506b81b3812ac900048059d2383126
//
// Cleaned version - https://github.com/kosua20/Rendu/blob/master/resources/common/shaders/screens/fxaa.frag
//
// Tweaks by mrDIMAS - https://github.com/FyroxEngine/Fyrox/blob/master/src/renderer/shaders/fxaa_fs.glsl

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screenTexture: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

// Trims the algorithm from processing darks.
#ifdef EDGE_THRESH_MIN_LOW
    const EDGE_THRESHOLD_MIN: f32 = 0.0833;
#endif

#ifdef EDGE_THRESH_MIN_MEDIUM
    const EDGE_THRESHOLD_MIN: f32 = 0.0625;
#endif

#ifdef EDGE_THRESH_MIN_HIGH
    const EDGE_THRESHOLD_MIN: f32 = 0.0312;
#endif

#ifdef EDGE_THRESH_MIN_ULTRA
    const EDGE_THRESHOLD_MIN: f32 = 0.0156;
#endif

#ifdef EDGE_THRESH_MIN_EXTREME
    const EDGE_THRESHOLD_MIN: f32 = 0.0078;
#endif

// The minimum amount of local contrast required to apply algorithm.
#ifdef EDGE_THRESH_LOW
    const EDGE_THRESHOLD_MAX: f32 = 0.250;
#endif

#ifdef EDGE_THRESH_MEDIUM
    const EDGE_THRESHOLD_MAX: f32 = 0.166;
#endif

#ifdef EDGE_THRESH_HIGH
    const EDGE_THRESHOLD_MAX: f32 = 0.125;
#endif

#ifdef EDGE_THRESH_ULTRA
    const EDGE_THRESHOLD_MAX: f32 = 0.063;
#endif

#ifdef EDGE_THRESH_EXTREME
    const EDGE_THRESHOLD_MAX: f32 = 0.031;
#endif

const ITERATIONS: i32 = 12; //default is 12
const SUBPIXEL_QUALITY: f32 = 0.75;
// #define QUALITY(q) ((q) < 5 ? 1.0 : ((q) > 5 ? ((q) < 10 ? 2.0 : ((q) < 11 ? 4.0 : 8.0)) : 1.5))
fn QUALITY(q: i32) -> f32 {
    switch (q) {
        //case 0, 1, 2, 3, 4: { return 1.0; }
        default:              { return 1.0; }
        case 5:               { return 1.5; }
        case 6, 7, 8, 9:      { return 2.0; }
        case 10:              { return 4.0; }
        case 11:              { return 8.0; }
    }
}

fn rgb2luma(rgb: vec3<f32>) -> f32 {
    return sqrt(dot(rgb, vec3<f32>(0.299, 0.587, 0.114)));
}

// Performs FXAA post-process anti-aliasing as described in the Nvidia FXAA white paper and the associated shader code.
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(textureDimensions(screenTexture));
    let inverseScreenSize = 1.0 / resolution.xy;
    let texCoord = in.position.xy * inverseScreenSize;

    let centerSample = textureSampleLevel(screenTexture, samp, texCoord, 0.0);
    let colorCenter = centerSample.rgb;

    // Luma at the current fragment
    let lumaCenter = rgb2luma(colorCenter);

    // Luma at the four direct neighbors of the current fragment.
    let lumaDown = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(0, -1)).rgb);
    let lumaUp = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(0, 1)).rgb);
    let lumaLeft = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(-1, 0)).rgb);
    let lumaRight = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(1, 0)).rgb);

    // Find the maximum and minimum luma around the current fragment.
    let lumaMin = min(lumaCenter, min(min(lumaDown, lumaUp), min(lumaLeft, lumaRight)));
    let lumaMax = max(lumaCenter, max(max(lumaDown, lumaUp), max(lumaLeft, lumaRight)));

    // Compute the delta.
    let lumaRange = lumaMax - lumaMin;

    // If the luma variation is lower that a threshold (or if we are in a really dark area), we are not on an edge, don't perform any AA.
    if (lumaRange < max(EDGE_THRESHOLD_MIN, lumaMax * EDGE_THRESHOLD_MAX)) {
        return centerSample;
    }

    // Query the 4 remaining corners lumas.
    let lumaDownLeft  = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(-1, -1)).rgb);
    let lumaUpRight   = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(1, 1)).rgb);
    let lumaUpLeft    = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(-1, 1)).rgb);
    let lumaDownRight = rgb2luma(textureSampleLevel(screenTexture, samp, texCoord, 0.0, vec2<i32>(1, -1)).rgb);

    // Combine the four edges lumas (using intermediary variables for future computations with the same values).
    let lumaDownUp = lumaDown + lumaUp;
    let lumaLeftRight = lumaLeft + lumaRight;

    // Same for corners
    let lumaLeftCorners = lumaDownLeft + lumaUpLeft;
    let lumaDownCorners = lumaDownLeft + lumaDownRight;
    let lumaRightCorners = lumaDownRight + lumaUpRight;
    let lumaUpCorners = lumaUpRight + lumaUpLeft;

    // Compute an estimation of the gradient along the horizontal and vertical axis.
    let edgeHorizontal = abs(-2.0 * lumaLeft   + lumaLeftCorners)  + 
                         abs(-2.0 * lumaCenter + lumaDownUp) * 2.0 + 
                         abs(-2.0 * lumaRight  + lumaRightCorners);

    let edgeVertical =   abs(-2.0 * lumaUp     + lumaUpCorners)       + 
                         abs(-2.0 * lumaCenter + lumaLeftRight) * 2.0 + 
                         abs(-2.0 * lumaDown   + lumaDownCorners);

    // Is the local edge horizontal or vertical ?
    let isHorizontal = (edgeHorizontal >= edgeVertical);

    // Choose the step size (one pixel) accordingly.
    var stepLength = select(inverseScreenSize.x, inverseScreenSize.y, isHorizontal);

    // Select the two neighboring texels lumas in the opposite direction to the local edge.
    var luma1 = select(lumaLeft, lumaDown, isHorizontal);
    var luma2 = select(lumaRight, lumaUp, isHorizontal);

    // Compute gradients in this direction.
    let gradient1 = luma1 - lumaCenter;
    let gradient2 = luma2 - lumaCenter;

    // Which direction is the steepest ?
    let is1Steepest = abs(gradient1) >= abs(gradient2);

    // Gradient in the corresponding direction, normalized.
    let gradientScaled = 0.25 * max(abs(gradient1), abs(gradient2));

    // Average luma in the correct direction.
    var lumaLocalAverage = 0.0;
    if (is1Steepest) {
        // Switch the direction
        stepLength = -stepLength;
        lumaLocalAverage = 0.5 * (luma1 + lumaCenter);
    } else {
        lumaLocalAverage = 0.5 * (luma2 + lumaCenter);
    }

    // Shift UV in the correct direction by half a pixel.
    // Compute offset (for each iteration step) in the right direction.
    var currentUv = texCoord;
    var offset = vec2<f32>(0.0, 0.0);
    if (isHorizontal) {
        currentUv.y = currentUv.y + stepLength * 0.5;
        offset.x = inverseScreenSize.x;
    } else {
        currentUv.x = currentUv.x + stepLength * 0.5;
        offset.y = inverseScreenSize.y;
    }

    // Compute UVs to explore on each side of the edge, orthogonally. The QUALITY allows us to step faster.
    var uv1 = currentUv - offset; // * QUALITY(0); // (quality 0 is 1.0)
    var uv2 = currentUv + offset; // * QUALITY(0); // (quality 0 is 1.0)

    // Read the lumas at both current extremities of the exploration segment, and compute the delta wrt to the local average luma.
    var lumaEnd1 = rgb2luma(textureSampleLevel(screenTexture, samp, uv1, 0.0).rgb);
    var lumaEnd2 = rgb2luma(textureSampleLevel(screenTexture, samp, uv2, 0.0).rgb);
    lumaEnd1 = lumaEnd1 - lumaLocalAverage;
    lumaEnd2 = lumaEnd2 - lumaLocalAverage;

    // If the luma deltas at the current extremities is larger than the local gradient, we have reached the side of the edge.
    var reached1 = abs(lumaEnd1) >= gradientScaled;
    var reached2 = abs(lumaEnd2) >= gradientScaled;
    var reachedBoth = reached1 && reached2;

    // If the side is not reached, we continue to explore in this direction.
    uv1 = select(uv1 - offset, uv1, reached1); // * QUALITY(1); // (quality 1 is 1.0)
    uv2 = select(uv2 - offset, uv2, reached2); // * QUALITY(1); // (quality 1 is 1.0)

    // If both sides have not been reached, continue to explore.
    if (!reachedBoth) {
        for (var i: i32 = 2; i < ITERATIONS; i = i + 1) {
            // If needed, read luma in 1st direction, compute delta.
            if (!reached1) { 
                lumaEnd1 = rgb2luma(textureSampleLevel(screenTexture, samp, uv1, 0.0).rgb);
                lumaEnd1 = lumaEnd1 - lumaLocalAverage;
            }
            // If needed, read luma in opposite direction, compute delta.
            if (!reached2) { 
                lumaEnd2 = rgb2luma(textureSampleLevel(screenTexture, samp, uv2, 0.0).rgb);
                lumaEnd2 = lumaEnd2 - lumaLocalAverage;
            }
            // If the luma deltas at the current extremities is larger than the local gradient, we have reached the side of the edge.
            reached1 = abs(lumaEnd1) >= gradientScaled;
            reached2 = abs(lumaEnd2) >= gradientScaled;
            reachedBoth = reached1 && reached2;

            // If the side is not reached, we continue to explore in this direction, with a variable quality.
            if (!reached1) {
                uv1 = uv1 - offset * QUALITY(i);
            }
            if (!reached2) {
                uv2 = uv2 + offset * QUALITY(i);
            }

            // If both sides have been reached, stop the exploration.
            if (reachedBoth) { 
                break; 
            }
        }
    }

    // Compute the distances to each side edge of the edge (!).
    var distance1 = select(texCoord.y - uv1.y, texCoord.x - uv1.x, isHorizontal);
    var distance2 = select(uv2.y - texCoord.y, uv2.x - texCoord.x, isHorizontal);

    // In which direction is the side of the edge closer ?
    let isDirection1 = distance1 < distance2;
    let distanceFinal = min(distance1, distance2);

    // Thickness of the edge.
    let edgeThickness = (distance1 + distance2);

    // Is the luma at center smaller than the local average ?
    let isLumaCenterSmaller = lumaCenter < lumaLocalAverage;

    // If the luma at center is smaller than at its neighbor, the delta luma at each end should be positive (same variation).
    let correctVariation1 = (lumaEnd1 < 0.0) != isLumaCenterSmaller;
    let correctVariation2 = (lumaEnd2 < 0.0) != isLumaCenterSmaller;

    // Only keep the result in the direction of the closer side of the edge.
    var correctVariation = select(correctVariation2, correctVariation1, isDirection1);

    // UV offset: read in the direction of the closest side of the edge.
    let pixelOffset = - distanceFinal / edgeThickness + 0.5;

    // If the luma variation is incorrect, do not offset.
    var finalOffset = select(0.0, pixelOffset, correctVariation);

    // Sub-pixel shifting
    // Full weighted average of the luma over the 3x3 neighborhood.
    let lumaAverage = (1.0 / 12.0) * (2.0 * (lumaDownUp + lumaLeftRight) + lumaLeftCorners + lumaRightCorners);
    // Ratio of the delta between the global average and the center luma, over the luma range in the 3x3 neighborhood.
    let subPixelOffset1 = clamp(abs(lumaAverage - lumaCenter) / lumaRange, 0.0, 1.0);
    let subPixelOffset2 = (-2.0 * subPixelOffset1 + 3.0) * subPixelOffset1 * subPixelOffset1;
    // Compute a sub-pixel offset based on this delta.
    let subPixelOffsetFinal = subPixelOffset2 * subPixelOffset2 * SUBPIXEL_QUALITY;

    // Pick the biggest of the two offsets.
    finalOffset = max(finalOffset, subPixelOffsetFinal);

    // Compute the final UV coordinates.
    var finalUv = texCoord;
    if (isHorizontal) {
        finalUv.y = finalUv.y + finalOffset * stepLength;
    } else {
        finalUv.x = finalUv.x + finalOffset * stepLength;
    }

    // Read the color at the new UV coordinates, and use it.
    var finalColor = textureSampleLevel(screenTexture, samp, finalUv, 0.0).rgb;
    return vec4<f32>(finalColor, centerSample.a);
}
