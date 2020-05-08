#version 450

// pathfinder/shaders/tile.fs.glsl
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//      Mask UV 0         Mask UV 1
//          +                 +
//          |                 |
//    +-----v-----+     +-----v-----+
//    |           | MIN |           |
//    |  Mask  0  +----->  Mask  1  +------+
//    |           |     |           |      |
//    +-----------+     +-----------+      v       +-------------+
//                                       Apply     |             |       GPU
//                                       Mask +---->  Composite  +---->Blender
//                                         ^       |             |
//    +-----------+     +-----------+      |       +-------------+
//    |           |     |           |      |
//    |  Color 0  +----->  Color 1  +------+
//    |  Filter   |  ×  |           |
//    |           |     |           |
//    +-----^-----+     +-----^-----+
//          |                 |
//          +                 +
//     Color UV 0        Color UV 1

#extension GL_GOOGLE_include_directive : enable

precision highp float;
precision highp sampler2D;

#define EPSILON     0.00001

#define FRAC_6_PI   1.9098593171027443
#define FRAC_PI_3   1.0471975511965976

#define TILE_CTRL_MASK_MASK                     0x3
#define TILE_CTRL_MASK_WINDING                  0x1
#define TILE_CTRL_MASK_EVEN_ODD                 0x2

#define TILE_CTRL_MASK_0_SHIFT                  0

#define COMBINER_CTRL_COLOR_COMBINE_MASK        0x3
#define COMBINER_CTRL_COLOR_COMBINE_SRC_IN      0x1
#define COMBINER_CTRL_COLOR_COMBINE_DEST_IN     0x2

#define COMBINER_CTRL_FILTER_MASK               0x3
#define COMBINER_CTRL_FILTER_RADIAL_GRADIENT    0x1
#define COMBINER_CTRL_FILTER_TEXT               0x2
#define COMBINER_CTRL_FILTER_BLUR               0x3

#define COMBINER_CTRL_COMPOSITE_MASK            0xf
#define COMBINER_CTRL_COMPOSITE_NORMAL          0x0
#define COMBINER_CTRL_COMPOSITE_MULTIPLY        0x1
#define COMBINER_CTRL_COMPOSITE_SCREEN          0x2
#define COMBINER_CTRL_COMPOSITE_OVERLAY         0x3
#define COMBINER_CTRL_COMPOSITE_DARKEN          0x4
#define COMBINER_CTRL_COMPOSITE_LIGHTEN         0x5
#define COMBINER_CTRL_COMPOSITE_COLOR_DODGE     0x6
#define COMBINER_CTRL_COMPOSITE_COLOR_BURN      0x7
#define COMBINER_CTRL_COMPOSITE_HARD_LIGHT      0x8
#define COMBINER_CTRL_COMPOSITE_SOFT_LIGHT      0x9
#define COMBINER_CTRL_COMPOSITE_DIFFERENCE      0xa
#define COMBINER_CTRL_COMPOSITE_EXCLUSION       0xb
#define COMBINER_CTRL_COMPOSITE_HUE             0xc
#define COMBINER_CTRL_COMPOSITE_SATURATION      0xd
#define COMBINER_CTRL_COMPOSITE_COLOR           0xe
#define COMBINER_CTRL_COMPOSITE_LUMINOSITY      0xf

#define COMBINER_CTRL_COLOR_FILTER_SHIFT        4
#define COMBINER_CTRL_COLOR_COMBINE_SHIFT       6
#define COMBINER_CTRL_COMPOSITE_SHIFT           8

layout(set=1, binding=0) uniform sampler textureSampler;
layout(set=1, binding=1) uniform texture2D uColorTexture0;
layout(set=1, binding=2) uniform texture2D uMaskTexture0;
layout(set=1, binding=3) uniform texture2D uDestTexture;
layout(set=1, binding=4) uniform texture2D uGammaLUT;

layout(set=2, binding=0) uniform uFilterParams0 {
    vec4 filterParams0; 
};
layout(set=2, binding=1) uniform uFilterParams1 {
    vec4 filterParams1; 
};
layout(set=2, binding=2) uniform uFilterParams2 {
    vec4 filterParams2; 
};
layout(set=2, binding=3) uniform uFramebufferSize {
    vec2 framebufferSize; 
};
layout(set=2, binding=4) uniform uColorTexture0Size {
    vec2 colorTexture0Size; 
};
layout(set=2, binding=5) uniform uCtrl {
    int ctrl;
};

in vec3 vMaskTexCoord0;
in vec2 vColorTexCoord0;
in vec4 vBaseColor;
in float vTileCtrl;

out vec4 oFragColor;

// Color sampling

vec4 sampleColor(texture2D colorTexture, vec2 colorTexCoord) {
    return texture(sampler2D(colorTexture, textureSampler), colorTexCoord);
}

// Color combining

vec4 combineColor0(vec4 destColor, vec4 srcColor, int op) {
    switch (op) {
    case COMBINER_CTRL_COLOR_COMBINE_SRC_IN:
        return vec4(srcColor.rgb, srcColor.a * destColor.a);
    case COMBINER_CTRL_COLOR_COMBINE_DEST_IN:
        return vec4(destColor.rgb, srcColor.a * destColor.a);
    }
    return destColor;
}

// Text filter

float filterTextSample1Tap(float offset, texture2D colorTexture, vec2 colorTexCoord) {
    return texture(sampler2D(colorTexture, textureSampler), colorTexCoord + vec2(offset, 0.0)).r;
}

// Samples 9 taps around the current pixel.
void filterTextSample9Tap(out vec4 outAlphaLeft,
                          out float outAlphaCenter,
                          out vec4 outAlphaRight,
                          texture2D colorTexture,
                          vec2 colorTexCoord,
                          vec4 kernel,
                          float onePixel) {
    bool wide = kernel.x > 0.0;
    outAlphaLeft =
        vec4(wide ? filterTextSample1Tap(-4.0 * onePixel, colorTexture, colorTexCoord) : 0.0,
             filterTextSample1Tap(-3.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(-2.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(-1.0 * onePixel, colorTexture, colorTexCoord));
    outAlphaCenter = filterTextSample1Tap(0.0, colorTexture, colorTexCoord);
    outAlphaRight =
        vec4(filterTextSample1Tap(1.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(2.0 * onePixel, colorTexture, colorTexCoord),
             filterTextSample1Tap(3.0 * onePixel, colorTexture, colorTexCoord),
             wide ? filterTextSample1Tap(4.0 * onePixel, colorTexture, colorTexCoord) : 0.0);
}

float filterTextConvolve7Tap(vec4 alpha0, vec3 alpha1, vec4 kernel) {
    return dot(alpha0, kernel) + dot(alpha1, kernel.zyx);
}

float filterTextGammaCorrectChannel(float bgColor, float fgColor, texture2D gammaLUT) {
    return texture(sampler2D(gammaLUT, textureSampler), vec2(fgColor, 1.0 - bgColor)).r;
}

// `fgColor` is in linear space.
vec3 filterTextGammaCorrect(vec3 bgColor, vec3 fgColor, texture2D gammaLUT) {
    return vec3(filterTextGammaCorrectChannel(bgColor.r, fgColor.r, gammaLUT),
                filterTextGammaCorrectChannel(bgColor.g, fgColor.g, gammaLUT),
                filterTextGammaCorrectChannel(bgColor.b, fgColor.b, gammaLUT));
}

//                | x          y          z          w
//  --------------+--------------------------------------------------------
//  filterParams0 | kernel[0]  kernel[1]  kernel[2]  kernel[3]
//  filterParams1 | bgColor.r  bgColor.g  bgColor.b  -
//  filterParams2 | fgColor.r  fgColor.g  fgColor.b  gammaCorrectionEnabled
vec4 filterText(vec2 colorTexCoord,
                texture2D colorTexture,
                texture2D gammaLUT,
                vec2 colorTextureSize,
                vec4 filterParams0,
                vec4 filterParams1,
                vec4 filterParams2) {
    // Unpack.
    vec4 kernel = filterParams0;
    vec3 bgColor = filterParams1.rgb;
    vec3 fgColor = filterParams2.rgb;
    bool gammaCorrectionEnabled = filterParams2.a != 0.0;

    // Apply defringing if necessary.
    vec3 alpha;
    if (kernel.w == 0.0) {
        alpha = texture(sampler2D(colorTexture, textureSampler), colorTexCoord).rrr;
    } else {
        vec4 alphaLeft, alphaRight;
        float alphaCenter;
        filterTextSample9Tap(alphaLeft,
                             alphaCenter,
                             alphaRight,
                             colorTexture,
                             colorTexCoord,
                             kernel,
                             1.0 / colorTextureSize.x);

        float r = filterTextConvolve7Tap(alphaLeft, vec3(alphaCenter, alphaRight.xy), kernel);
        float g = filterTextConvolve7Tap(vec4(alphaLeft.yzw, alphaCenter), alphaRight.xyz, kernel);
        float b = filterTextConvolve7Tap(vec4(alphaLeft.zw, alphaCenter, alphaRight.x),
                                         alphaRight.yzw,
                                         kernel);

        alpha = vec3(r, g, b);
    }

    // Apply gamma correction if necessary.
    if (gammaCorrectionEnabled)
        alpha = filterTextGammaCorrect(bgColor, alpha, gammaLUT);

    // Finish.
    return vec4(mix(bgColor, fgColor, alpha), 1.0);
}

// Other filters

// This is based on Pixman (MIT license). Copy and pasting the excellent comment
// from there:

// Implementation of radial gradients following the PDF specification.
// See section 8.7.4.5.4 Type 3 (Radial) Shadings of the PDF Reference
// Manual (PDF 32000-1:2008 at the time of this writing).
//
// In the radial gradient problem we are given two circles (c₁,r₁) and
// (c₂,r₂) that define the gradient itself.
//
// Mathematically the gradient can be defined as the family of circles
//
//     ((1-t)·c₁ + t·(c₂), (1-t)·r₁ + t·r₂)
//
// excluding those circles whose radius would be < 0. When a point
// belongs to more than one circle, the one with a bigger t is the only
// one that contributes to its color. When a point does not belong
// to any of the circles, it is transparent black, i.e. RGBA (0, 0, 0, 0).
// Further limitations on the range of values for t are imposed when
// the gradient is not repeated, namely t must belong to [0,1].
//
// The graphical result is the same as drawing the valid (radius > 0)
// circles with increasing t in [-∞, +∞] (or in [0,1] if the gradient
// is not repeated) using SOURCE operator composition.
//
// It looks like a cone pointing towards the viewer if the ending circle
// is smaller than the starting one, a cone pointing inside the page if
// the starting circle is the smaller one and like a cylinder if they
// have the same radius.
//
// What we actually do is, given the point whose color we are interested
// in, compute the t values for that point, solving for t in:
//
//     length((1-t)·c₁ + t·(c₂) - p) = (1-t)·r₁ + t·r₂
//
// Let's rewrite it in a simpler way, by defining some auxiliary
// variables:
//
//     cd = c₂ - c₁
//     pd = p - c₁
//     dr = r₂ - r₁
//     length(t·cd - pd) = r₁ + t·dr
//
// which actually means
//
//     hypot(t·cdx - pdx, t·cdy - pdy) = r₁ + t·dr
//
// or
//
//     ⎷((t·cdx - pdx)² + (t·cdy - pdy)²) = r₁ + t·dr.
//
// If we impose (as stated earlier) that r₁ + t·dr ≥ 0, it becomes:
//
//     (t·cdx - pdx)² + (t·cdy - pdy)² = (r₁ + t·dr)²
//
// where we can actually expand the squares and solve for t:
//
//     t²cdx² - 2t·cdx·pdx + pdx² + t²cdy² - 2t·cdy·pdy + pdy² =
//       = r₁² + 2·r₁·t·dr + t²·dr²
//
//     (cdx² + cdy² - dr²)t² - 2(cdx·pdx + cdy·pdy + r₁·dr)t +
//         (pdx² + pdy² - r₁²) = 0
//
//     A = cdx² + cdy² - dr²
//     B = pdx·cdx + pdy·cdy + r₁·dr
//     C = pdx² + pdy² - r₁²
//     At² - 2Bt + C = 0
//
// The solutions (unless the equation degenerates because of A = 0) are:
//
//     t = (B ± ⎷(B² - A·C)) / A
//
// The solution we are going to prefer is the bigger one, unless the
// radius associated to it is negative (or it falls outside the valid t
// range).
//
// Additional observations (useful for optimizations):
// A does not depend on p
//
// A < 0 ⟺ one of the two circles completely contains the other one
//   ⟺ for every p, the radii associated with the two t solutions have
//       opposite sign
//
//                | x           y           z               w
//  --------------+-----------------------------------------------------
//  filterParams0 | lineFrom.x  lineFrom.y  lineVector.x    lineVector.y
//  filterParams1 | radii.x     radii.y     uvOrigin.x      uvOrigin.y
//  filterParams2 | -           -           -               -
vec4 filterRadialGradient(vec2 colorTexCoord,
                          texture2D colorTexture,
                          vec2 colorTextureSize,
                          vec2 fragCoord,
                          vec2 framebufferSize,
                          vec4 filterParams0,
                          vec4 filterParams1) {
    vec2 lineFrom = filterParams0.xy, lineVector = filterParams0.zw;
    vec2 radii = filterParams1.xy, uvOrigin = filterParams1.zw;

    vec2 dP = colorTexCoord - lineFrom, dC = lineVector;
    float dR = radii.y - radii.x;

    float a = dot(dC, dC) - dR * dR;
    float b = dot(dP, dC) + radii.x * dR;
    float c = dot(dP, dP) - radii.x * radii.x;
    float discrim = b * b - a * c;

    vec4 color = vec4(0.0);
    if (abs(discrim) >= EPSILON) {
        vec2 ts = vec2(sqrt(discrim) * vec2(1.0, -1.0) + vec2(b)) / vec2(a);
        if (ts.x > ts.y)
            ts = ts.yx;
        float t = ts.x >= 0.0 ? ts.x : ts.y;
        color = texture(sampler2D(colorTexture, textureSampler), uvOrigin + vec2(clamp(t, 0.0, 1.0), 0.0));
    }

    return color;
}

//                | x             y             z             w
//  --------------+----------------------------------------------------
//  filterParams0 | srcOffset.x   srcOffset.y   support       -
//  filterParams1 | gaussCoeff.x  gaussCoeff.y  gaussCoeff.z  -
//  filterParams2 | -             -                 -             -
vec4 filterBlur(vec2 colorTexCoord,
                texture2D colorTexture,
                vec2 colorTextureSize,
                vec4 filterParams0,
                vec4 filterParams1) {
    // Unpack.
    vec2 srcOffsetScale = filterParams0.xy / colorTextureSize;
    int support = int(filterParams0.z);
    vec3 gaussCoeff = filterParams1.xyz;

    // Set up our incremental calculation.
    float gaussSum = gaussCoeff.x;
    vec4 color = texture(sampler2D(colorTexture, textureSampler), colorTexCoord) * gaussCoeff.x;
    gaussCoeff.xy *= gaussCoeff.yz;

    // This is a common trick that lets us use the texture filtering hardware to evaluate two
    // texels at a time. The basic principle is that, if c0 and c1 are colors of adjacent texels
    // and k0 and k1 are arbitrary factors, the formula `k0 * c0 + k1 * c1` is equivalent to
    // `(k0 + k1) * lerp(c0, c1, k1 / (k0 + k1))`. Linear interpolation, as performed by the
    // texturing hardware when sampling adjacent pixels in one direction, evaluates
    // `lerp(c0, c1, t)` where t is the offset from the texel with color `c0`. To evaluate the
    // formula `k0 * c0 + k1 * c1`, therefore, we can use the texture hardware to perform linear
    // interpolation with `t = k1 / (k0 + k1)`.
    for (int i = 1; i <= support; i += 2) {
        float gaussPartialSum = gaussCoeff.x;
        gaussCoeff.xy *= gaussCoeff.yz;
        gaussPartialSum += gaussCoeff.x;

        vec2 srcOffset = srcOffsetScale * (float(i) + gaussCoeff.x / gaussPartialSum);
        color += (texture(sampler2D(colorTexture, textureSampler), colorTexCoord - srcOffset) +
                  texture(sampler2D(colorTexture, textureSampler), colorTexCoord + srcOffset)) * gaussPartialSum;

        gaussSum += 2.0 * gaussPartialSum;
        gaussCoeff.xy *= gaussCoeff.yz;
    }

    // Finish.
    return color / gaussSum;
}

vec4 filterNone(vec2 colorTexCoord, texture2D colorTexture) {
    return sampleColor(colorTexture, colorTexCoord);
}

vec4 filterColor(vec2 colorTexCoord,
                 texture2D colorTexture,
                 texture2D gammaLUT,
                 vec2 colorTextureSize,
                 vec2 fragCoord,
                 vec2 framebufferSize,
                 vec4 filterParams0,
                 vec4 filterParams1,
                 vec4 filterParams2,
                 int colorFilter) {
    switch (colorFilter) {
    case COMBINER_CTRL_FILTER_RADIAL_GRADIENT:
        return filterRadialGradient(colorTexCoord,
                                    colorTexture,
                                    colorTextureSize,
                                    fragCoord,
                                    framebufferSize,
                                    filterParams0,
                                    filterParams1);
    case COMBINER_CTRL_FILTER_BLUR:
        return filterBlur(colorTexCoord,
                          colorTexture,
                          colorTextureSize,
                          filterParams0,
                          filterParams1);
    case COMBINER_CTRL_FILTER_TEXT:
        return filterText(colorTexCoord,
                          colorTexture,
                          gammaLUT,
                          colorTextureSize,
                          filterParams0,
                          filterParams1,
                          filterParams2);
    }
    return filterNone(colorTexCoord, colorTexture);
}

// Compositing

vec3 compositeSelect(bvec3 cond, vec3 ifTrue, vec3 ifFalse) {
    return vec3(cond.x ? ifTrue.x : ifFalse.x,
                cond.y ? ifTrue.y : ifFalse.y,
                cond.z ? ifTrue.z : ifFalse.z);
}

float compositeDivide(float num, float denom) {
    return denom != 0.0 ? num / denom : 0.0;
}

vec3 compositeColorDodge(vec3 destColor, vec3 srcColor) {
    bvec3 destZero = equal(destColor, vec3(0.0)), srcOne = equal(srcColor, vec3(1.0));
    return compositeSelect(destZero,
                           vec3(0.0),
                           compositeSelect(srcOne, vec3(1.0), destColor / (vec3(1.0) - srcColor)));
}

// https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_RGB_alternative
vec3 compositeHSLToRGB(vec3 hsl) {
    float a = hsl.y * min(hsl.z, 1.0 - hsl.z);
    vec3 ks = mod(vec3(0.0, 8.0, 4.0) + vec3(hsl.x * FRAC_6_PI), 12.0);
    return hsl.zzz - clamp(min(ks - vec3(3.0), vec3(9.0) - ks), -1.0, 1.0) * a;
}

// https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
vec3 compositeRGBToHSL(vec3 rgb) {
    float v = max(max(rgb.r, rgb.g), rgb.b), xMin = min(min(rgb.r, rgb.g), rgb.b);
    float c = v - xMin, l = mix(xMin, v, 0.5);
    vec3 terms = rgb.r == v ? vec3(0.0, rgb.gb) :
                 rgb.g == v ? vec3(2.0, rgb.br) :
                              vec3(4.0, rgb.rg);
    float h = FRAC_PI_3 * compositeDivide(terms.x * c + terms.y - terms.z, c);
    float s = compositeDivide(c, v);
    return vec3(h, s, l);
}

vec3 compositeScreen(vec3 destColor, vec3 srcColor) {
    return destColor + srcColor - destColor * srcColor;
}

vec3 compositeHardLight(vec3 destColor, vec3 srcColor) {
    return compositeSelect(lessThanEqual(srcColor, vec3(0.5)),
                           destColor * vec3(2.0) * srcColor,
                           compositeScreen(destColor, vec3(2.0) * srcColor - vec3(1.0)));
}

vec3 compositeSoftLight(vec3 destColor, vec3 srcColor) {
    vec3 darkenedDestColor =
        compositeSelect(lessThanEqual(destColor, vec3(0.25)),
                        ((vec3(16.0) * destColor - 12.0) * destColor + 4.0) * destColor,
                        sqrt(destColor));
    vec3 factor = compositeSelect(lessThanEqual(srcColor, vec3(0.5)),
                                  destColor * (vec3(1.0) - destColor),
                                  darkenedDestColor - destColor);
    return destColor + (srcColor * 2.0 - 1.0) * factor;
}

vec3 compositeHSL(vec3 destColor, vec3 srcColor, int op) {
    switch (op) {
    case COMBINER_CTRL_COMPOSITE_HUE:
        return vec3(srcColor.x,  destColor.y, destColor.z);
    case COMBINER_CTRL_COMPOSITE_SATURATION:
        return vec3(destColor.x, srcColor.y,  destColor.z);
    case COMBINER_CTRL_COMPOSITE_COLOR:
        return vec3(srcColor.x,  srcColor.y,  destColor.z);
    default:
        return vec3(destColor.x, destColor.y, srcColor.z);
    }
}

vec3 compositeRGB(vec3 destColor, vec3 srcColor, int op) {
    switch (op) {
    case COMBINER_CTRL_COMPOSITE_MULTIPLY:
        return destColor * srcColor;
    case COMBINER_CTRL_COMPOSITE_SCREEN:
        return compositeScreen(destColor, srcColor);
    case COMBINER_CTRL_COMPOSITE_OVERLAY:
        return compositeHardLight(srcColor, destColor);
    case COMBINER_CTRL_COMPOSITE_DARKEN:
        return min(destColor, srcColor);
    case COMBINER_CTRL_COMPOSITE_LIGHTEN:
        return max(destColor, srcColor);
    case COMBINER_CTRL_COMPOSITE_COLOR_DODGE:
        return compositeColorDodge(destColor, srcColor);
    case COMBINER_CTRL_COMPOSITE_COLOR_BURN:
        return vec3(1.0) - compositeColorDodge(vec3(1.0) - destColor, vec3(1.0) - srcColor);
    case COMBINER_CTRL_COMPOSITE_HARD_LIGHT:
        return compositeHardLight(destColor, srcColor);
    case COMBINER_CTRL_COMPOSITE_SOFT_LIGHT:
        return compositeSoftLight(destColor, srcColor);
    case COMBINER_CTRL_COMPOSITE_DIFFERENCE:
        return abs(destColor - srcColor);
    case COMBINER_CTRL_COMPOSITE_EXCLUSION:
        return destColor + srcColor - vec3(2.0) * destColor * srcColor;
    case COMBINER_CTRL_COMPOSITE_HUE:
    case COMBINER_CTRL_COMPOSITE_SATURATION:
    case COMBINER_CTRL_COMPOSITE_COLOR:
    case COMBINER_CTRL_COMPOSITE_LUMINOSITY:
        return compositeHSLToRGB(compositeHSL(compositeRGBToHSL(destColor),
                                              compositeRGBToHSL(srcColor),
                                              op));
    }
    return srcColor;
}

vec4 composite(vec4 srcColor,
               texture2D destTexture,
               vec2 destTextureSize,
               vec2 fragCoord,
               int op) {
    if (op == COMBINER_CTRL_COMPOSITE_NORMAL)
        return srcColor;

    // FIXME(pcwalton): What should the output alpha be here?
    vec2 destTexCoord = fragCoord / destTextureSize;
    vec4 destColor = texture(sampler2D(destTexture, textureSampler), destTexCoord);
    vec3 blendedRGB = compositeRGB(destColor.rgb, srcColor.rgb, op);
    return vec4(srcColor.a * (1.0 - destColor.a) * srcColor.rgb +
                srcColor.a * destColor.a * blendedRGB +
                (1.0 - srcColor.a) * destColor.rgb,
                1.0);
}

// Masks
float sampleMask(float maskAlpha,
                 texture2D maskTexture,
                 vec3 maskTexCoord,
                 int maskCtrl) {
    if (maskCtrl == 0)
        return maskAlpha;
    float coverage = texture(sampler2D(maskTexture, textureSampler), maskTexCoord.xy).r + maskTexCoord.z;
    if ((maskCtrl & TILE_CTRL_MASK_WINDING) != 0)
        coverage = abs(coverage);
    else
        coverage = 1.0 - abs(1.0 - mod(coverage, 2.0));
    return min(maskAlpha, coverage);
}

// Main function

void calculateColor(int tileCtrl, int ctrl) {
    // Sample mask.
    int maskCtrl0 = (tileCtrl >> TILE_CTRL_MASK_0_SHIFT) & TILE_CTRL_MASK_MASK;
    float maskAlpha = 1.0;
    maskAlpha = sampleMask(maskAlpha, uMaskTexture0, vMaskTexCoord0, maskCtrl0);

    // Sample color.
    vec4 color = vBaseColor;
    int color0Combine = (ctrl >> COMBINER_CTRL_COLOR_COMBINE_SHIFT) &
        COMBINER_CTRL_COLOR_COMBINE_MASK;
    if (color0Combine != 0) {
        int color0Filter = (ctrl >> COMBINER_CTRL_COLOR_FILTER_SHIFT) & COMBINER_CTRL_FILTER_MASK;
        vec4 color0 = filterColor(vColorTexCoord0,
                                  uColorTexture0,
                                  uGammaLUT,
                                  colorTexture0Size,
                                  gl_FragCoord.xy,
                                  framebufferSize,
                                  filterParams0,
                                  filterParams1,
                                  filterParams2,
                                  color0Filter);
        color = combineColor0(color, color0, color0Combine);
    }

    // Apply mask.
    color.a *= maskAlpha;

    // Apply composite.
    int compositeOp = (ctrl >> COMBINER_CTRL_COMPOSITE_SHIFT) & COMBINER_CTRL_COMPOSITE_MASK;
    color = composite(color, uDestTexture, framebufferSize, gl_FragCoord.xy, compositeOp);

    // Premultiply alpha.
    color.rgb *= color.a;
    oFragColor = color;
}

// Entry point
//
// TODO(pcwalton): Generate this dynamically.

void main() {
    calculateColor(int(vTileCtrl), ctrl);
}
