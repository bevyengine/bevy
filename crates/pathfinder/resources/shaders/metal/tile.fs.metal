// Automatically generated from files in pathfinder/shaders/. Do not edit!
#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct uMaskTextureSize0
{
    float2 iMaskTextureSize0;
};

struct uColorTextureSize0
{
    float2 iColorTextureSize0;
};

struct uFramebufferSize
{
    float2 iFramebufferSize;
};

struct uFilterParams0
{
    float4 iFilterParams0;
};

struct uFilterParams1
{
    float4 iFilterParams1;
};

struct uFilterParams2
{
    float4 iFilterParams2;
};

struct uCtrl
{
    int iCtrl;
};

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

struct main0_in
{
    float3 vMaskTexCoord0 [[user(locn0)]];
    float2 vColorTexCoord0 [[user(locn1)]];
    float4 vBaseColor [[user(locn2)]];
    float vTileCtrl [[user(locn3)]];
};

// Implementation of the GLSL mod() function, which is slightly different than Metal fmod()
template<typename Tx, typename Ty>
inline Tx mod(Tx x, Ty y)
{
    return x - y * floor(x / y);
}

static inline __attribute__((always_inline))
float sampleMask(thread const float& maskAlpha, thread const texture2d<float> maskTexture, thread const float2& maskTextureSize, thread const float3& maskTexCoord, thread const int& maskCtrl, thread sampler uSampler)
{
    if (maskCtrl == 0)
    {
        return maskAlpha;
    }
    int2 maskTexCoordI = int2(floor(maskTexCoord.xy));
    float4 texel = maskTexture.sample(uSampler, ((float2(maskTexCoordI / int2(1, 4)) + float2(0.5)) / maskTextureSize));
    float coverage = texel[maskTexCoordI.y % 4] + maskTexCoord.z;
    if ((maskCtrl & 1) != 0)
    {
        coverage = abs(coverage);
    }
    else
    {
        coverage = 1.0 - abs(1.0 - mod(coverage, 2.0));
    }
    return fast::min(maskAlpha, coverage);
}

static inline __attribute__((always_inline))
float4 filterRadialGradient(thread const float2& colorTexCoord, thread const texture2d<float> colorTexture, thread const float2& colorTextureSize, thread const float2& fragCoord, thread const float2& framebufferSize, thread const float4& filterParams0, thread const float4& filterParams1, thread sampler uSampler)
{
    float2 lineFrom = filterParams0.xy;
    float2 lineVector = filterParams0.zw;
    float2 radii = filterParams1.xy;
    float2 uvOrigin = filterParams1.zw;
    float2 dP = colorTexCoord - lineFrom;
    float2 dC = lineVector;
    float dR = radii.y - radii.x;
    float a = dot(dC, dC) - (dR * dR);
    float b = dot(dP, dC) + (radii.x * dR);
    float c = dot(dP, dP) - (radii.x * radii.x);
    float discrim = (b * b) - (a * c);
    float4 color = float4(0.0);
    if (abs(discrim) >= 9.9999997473787516355514526367188e-06)
    {
        float2 ts = float2((float2(1.0, -1.0) * sqrt(discrim)) + float2(b)) / float2(a);
        if (ts.x > ts.y)
        {
            ts = ts.yx;
        }
        float _566;
        if (ts.x >= 0.0)
        {
            _566 = ts.x;
        }
        else
        {
            _566 = ts.y;
        }
        float t = _566;
        color = colorTexture.sample(uSampler, (uvOrigin + float2(fast::clamp(t, 0.0, 1.0), 0.0)));
    }
    return color;
}

static inline __attribute__((always_inline))
float4 filterBlur(thread const float2& colorTexCoord, thread const texture2d<float> colorTexture, thread const float2& colorTextureSize, thread const float4& filterParams0, thread const float4& filterParams1, thread sampler uSampler)
{
    float2 srcOffsetScale = filterParams0.xy / colorTextureSize;
    int support = int(filterParams0.z);
    float3 gaussCoeff = filterParams1.xyz;
    float gaussSum = gaussCoeff.x;
    float4 color = colorTexture.sample(uSampler, colorTexCoord) * gaussCoeff.x;
    float2 _615 = gaussCoeff.xy * gaussCoeff.yz;
    gaussCoeff = float3(_615.x, _615.y, gaussCoeff.z);
    for (int i = 1; i <= support; i += 2)
    {
        float gaussPartialSum = gaussCoeff.x;
        float2 _635 = gaussCoeff.xy * gaussCoeff.yz;
        gaussCoeff = float3(_635.x, _635.y, gaussCoeff.z);
        gaussPartialSum += gaussCoeff.x;
        float2 srcOffset = srcOffsetScale * (float(i) + (gaussCoeff.x / gaussPartialSum));
        color += ((colorTexture.sample(uSampler, (colorTexCoord - srcOffset)) + colorTexture.sample(uSampler, (colorTexCoord + srcOffset))) * gaussPartialSum);
        gaussSum += (2.0 * gaussPartialSum);
        float2 _679 = gaussCoeff.xy * gaussCoeff.yz;
        gaussCoeff = float3(_679.x, _679.y, gaussCoeff.z);
    }
    return color / float4(gaussSum);
}

static inline __attribute__((always_inline))
float filterTextSample1Tap(thread const float& offset, thread const texture2d<float> colorTexture, thread const float2& colorTexCoord, thread sampler uSampler)
{
    return colorTexture.sample(uSampler, (colorTexCoord + float2(offset, 0.0))).x;
}

static inline __attribute__((always_inline))
void filterTextSample9Tap(thread float4& outAlphaLeft, thread float& outAlphaCenter, thread float4& outAlphaRight, thread const texture2d<float> colorTexture, thread const float2& colorTexCoord, thread const float4& kernel0, thread const float& onePixel, thread sampler uSampler)
{
    bool wide = kernel0.x > 0.0;
    float _243;
    if (wide)
    {
        float param = (-4.0) * onePixel;
        float2 param_1 = colorTexCoord;
        _243 = filterTextSample1Tap(param, colorTexture, param_1, uSampler);
    }
    else
    {
        _243 = 0.0;
    }
    float param_2 = (-3.0) * onePixel;
    float2 param_3 = colorTexCoord;
    float param_4 = (-2.0) * onePixel;
    float2 param_5 = colorTexCoord;
    float param_6 = (-1.0) * onePixel;
    float2 param_7 = colorTexCoord;
    outAlphaLeft = float4(_243, filterTextSample1Tap(param_2, colorTexture, param_3, uSampler), filterTextSample1Tap(param_4, colorTexture, param_5, uSampler), filterTextSample1Tap(param_6, colorTexture, param_7, uSampler));
    float param_8 = 0.0;
    float2 param_9 = colorTexCoord;
    outAlphaCenter = filterTextSample1Tap(param_8, colorTexture, param_9, uSampler);
    float param_10 = 1.0 * onePixel;
    float2 param_11 = colorTexCoord;
    float param_12 = 2.0 * onePixel;
    float2 param_13 = colorTexCoord;
    float param_14 = 3.0 * onePixel;
    float2 param_15 = colorTexCoord;
    float _303;
    if (wide)
    {
        float param_16 = 4.0 * onePixel;
        float2 param_17 = colorTexCoord;
        _303 = filterTextSample1Tap(param_16, colorTexture, param_17, uSampler);
    }
    else
    {
        _303 = 0.0;
    }
    outAlphaRight = float4(filterTextSample1Tap(param_10, colorTexture, param_11, uSampler), filterTextSample1Tap(param_12, colorTexture, param_13, uSampler), filterTextSample1Tap(param_14, colorTexture, param_15, uSampler), _303);
}

static inline __attribute__((always_inline))
float filterTextConvolve7Tap(thread const float4& alpha0, thread const float3& alpha1, thread const float4& kernel0)
{
    return dot(alpha0, kernel0) + dot(alpha1, kernel0.zyx);
}

static inline __attribute__((always_inline))
float filterTextGammaCorrectChannel(thread const float& bgColor, thread const float& fgColor, thread const texture2d<float> gammaLUT, thread sampler uSampler)
{
    return gammaLUT.sample(uSampler, float2(fgColor, 1.0 - bgColor)).x;
}

static inline __attribute__((always_inline))
float3 filterTextGammaCorrect(thread const float3& bgColor, thread const float3& fgColor, thread const texture2d<float> gammaLUT, thread sampler uSampler)
{
    float param = bgColor.x;
    float param_1 = fgColor.x;
    float param_2 = bgColor.y;
    float param_3 = fgColor.y;
    float param_4 = bgColor.z;
    float param_5 = fgColor.z;
    return float3(filterTextGammaCorrectChannel(param, param_1, gammaLUT, uSampler), filterTextGammaCorrectChannel(param_2, param_3, gammaLUT, uSampler), filterTextGammaCorrectChannel(param_4, param_5, gammaLUT, uSampler));
}

static inline __attribute__((always_inline))
float4 filterText(thread const float2& colorTexCoord, thread const texture2d<float> colorTexture, thread const texture2d<float> gammaLUT, thread const float2& colorTextureSize, thread const float4& filterParams0, thread const float4& filterParams1, thread const float4& filterParams2, thread sampler uSampler)
{
    float4 kernel0 = filterParams0;
    float3 bgColor = filterParams1.xyz;
    float3 fgColor = filterParams2.xyz;
    bool gammaCorrectionEnabled = filterParams2.w != 0.0;
    float3 alpha;
    if (kernel0.w == 0.0)
    {
        alpha = colorTexture.sample(uSampler, colorTexCoord).xxx;
    }
    else
    {
        float2 param_3 = colorTexCoord;
        float4 param_4 = kernel0;
        float param_5 = 1.0 / colorTextureSize.x;
        float4 param;
        float param_1;
        float4 param_2;
        filterTextSample9Tap(param, param_1, param_2, colorTexture, param_3, param_4, param_5, uSampler);
        float4 alphaLeft = param;
        float alphaCenter = param_1;
        float4 alphaRight = param_2;
        float4 param_6 = alphaLeft;
        float3 param_7 = float3(alphaCenter, alphaRight.xy);
        float4 param_8 = kernel0;
        float r = filterTextConvolve7Tap(param_6, param_7, param_8);
        float4 param_9 = float4(alphaLeft.yzw, alphaCenter);
        float3 param_10 = alphaRight.xyz;
        float4 param_11 = kernel0;
        float g = filterTextConvolve7Tap(param_9, param_10, param_11);
        float4 param_12 = float4(alphaLeft.zw, alphaCenter, alphaRight.x);
        float3 param_13 = alphaRight.yzw;
        float4 param_14 = kernel0;
        float b = filterTextConvolve7Tap(param_12, param_13, param_14);
        alpha = float3(r, g, b);
    }
    if (gammaCorrectionEnabled)
    {
        float3 param_15 = bgColor;
        float3 param_16 = alpha;
        alpha = filterTextGammaCorrect(param_15, param_16, gammaLUT, uSampler);
    }
    return float4(mix(bgColor, fgColor, alpha), 1.0);
}

static inline __attribute__((always_inline))
float4 sampleColor(thread const texture2d<float> colorTexture, thread const float2& colorTexCoord, thread sampler uSampler)
{
    return colorTexture.sample(uSampler, colorTexCoord);
}

static inline __attribute__((always_inline))
float4 filterNone(thread const float2& colorTexCoord, thread const texture2d<float> colorTexture, thread sampler uSampler)
{
    float2 param = colorTexCoord;
    return sampleColor(colorTexture, param, uSampler);
}

static inline __attribute__((always_inline))
float4 filterColor(thread const float2& colorTexCoord, thread const texture2d<float> colorTexture, thread const texture2d<float> gammaLUT, thread const float2& colorTextureSize, thread const float2& fragCoord, thread const float2& framebufferSize, thread const float4& filterParams0, thread const float4& filterParams1, thread const float4& filterParams2, thread const int& colorFilter, thread sampler uSampler)
{
    switch (colorFilter)
    {
        case 1:
        {
            float2 param = colorTexCoord;
            float2 param_1 = colorTextureSize;
            float2 param_2 = fragCoord;
            float2 param_3 = framebufferSize;
            float4 param_4 = filterParams0;
            float4 param_5 = filterParams1;
            return filterRadialGradient(param, colorTexture, param_1, param_2, param_3, param_4, param_5, uSampler);
        }
        case 3:
        {
            float2 param_6 = colorTexCoord;
            float2 param_7 = colorTextureSize;
            float4 param_8 = filterParams0;
            float4 param_9 = filterParams1;
            return filterBlur(param_6, colorTexture, param_7, param_8, param_9, uSampler);
        }
        case 2:
        {
            float2 param_10 = colorTexCoord;
            float2 param_11 = colorTextureSize;
            float4 param_12 = filterParams0;
            float4 param_13 = filterParams1;
            float4 param_14 = filterParams2;
            return filterText(param_10, colorTexture, gammaLUT, param_11, param_12, param_13, param_14, uSampler);
        }
    }
    float2 param_15 = colorTexCoord;
    return filterNone(param_15, colorTexture, uSampler);
}

static inline __attribute__((always_inline))
float4 combineColor0(thread const float4& destColor, thread const float4& srcColor, thread const int& op)
{
    switch (op)
    {
        case 1:
        {
            return float4(srcColor.xyz, srcColor.w * destColor.w);
        }
        case 2:
        {
            return float4(destColor.xyz, srcColor.w * destColor.w);
        }
    }
    return destColor;
}

static inline __attribute__((always_inline))
float3 compositeScreen(thread const float3& destColor, thread const float3& srcColor)
{
    return (destColor + srcColor) - (destColor * srcColor);
}

static inline __attribute__((always_inline))
float3 compositeSelect(thread const bool3& cond, thread const float3& ifTrue, thread const float3& ifFalse)
{
    float _745;
    if (cond.x)
    {
        _745 = ifTrue.x;
    }
    else
    {
        _745 = ifFalse.x;
    }
    float _756;
    if (cond.y)
    {
        _756 = ifTrue.y;
    }
    else
    {
        _756 = ifFalse.y;
    }
    float _767;
    if (cond.z)
    {
        _767 = ifTrue.z;
    }
    else
    {
        _767 = ifFalse.z;
    }
    return float3(_745, _756, _767);
}

static inline __attribute__((always_inline))
float3 compositeHardLight(thread const float3& destColor, thread const float3& srcColor)
{
    float3 param = destColor;
    float3 param_1 = (float3(2.0) * srcColor) - float3(1.0);
    bool3 param_2 = srcColor <= float3(0.5);
    float3 param_3 = (destColor * float3(2.0)) * srcColor;
    float3 param_4 = compositeScreen(param, param_1);
    return compositeSelect(param_2, param_3, param_4);
}

static inline __attribute__((always_inline))
float3 compositeColorDodge(thread const float3& destColor, thread const float3& srcColor)
{
    bool3 destZero = destColor == float3(0.0);
    bool3 srcOne = srcColor == float3(1.0);
    bool3 param = srcOne;
    float3 param_1 = float3(1.0);
    float3 param_2 = destColor / (float3(1.0) - srcColor);
    bool3 param_3 = destZero;
    float3 param_4 = float3(0.0);
    float3 param_5 = compositeSelect(param, param_1, param_2);
    return compositeSelect(param_3, param_4, param_5);
}

static inline __attribute__((always_inline))
float3 compositeSoftLight(thread const float3& destColor, thread const float3& srcColor)
{
    bool3 param = destColor <= float3(0.25);
    float3 param_1 = ((((float3(16.0) * destColor) - float3(12.0)) * destColor) + float3(4.0)) * destColor;
    float3 param_2 = sqrt(destColor);
    float3 darkenedDestColor = compositeSelect(param, param_1, param_2);
    bool3 param_3 = srcColor <= float3(0.5);
    float3 param_4 = destColor * (float3(1.0) - destColor);
    float3 param_5 = darkenedDestColor - destColor;
    float3 factor = compositeSelect(param_3, param_4, param_5);
    return destColor + (((srcColor * 2.0) - float3(1.0)) * factor);
}

static inline __attribute__((always_inline))
float compositeDivide(thread const float& num, thread const float& denom)
{
    float _781;
    if (denom != 0.0)
    {
        _781 = num / denom;
    }
    else
    {
        _781 = 0.0;
    }
    return _781;
}

static inline __attribute__((always_inline))
float3 compositeRGBToHSL(thread const float3& rgb)
{
    float v = fast::max(fast::max(rgb.x, rgb.y), rgb.z);
    float xMin = fast::min(fast::min(rgb.x, rgb.y), rgb.z);
    float c = v - xMin;
    float l = mix(xMin, v, 0.5);
    float3 _887;
    if (rgb.x == v)
    {
        _887 = float3(0.0, rgb.yz);
    }
    else
    {
        float3 _900;
        if (rgb.y == v)
        {
            _900 = float3(2.0, rgb.zx);
        }
        else
        {
            _900 = float3(4.0, rgb.xy);
        }
        _887 = _900;
    }
    float3 terms = _887;
    float param = ((terms.x * c) + terms.y) - terms.z;
    float param_1 = c;
    float h = 1.0471975803375244140625 * compositeDivide(param, param_1);
    float param_2 = c;
    float param_3 = v;
    float s = compositeDivide(param_2, param_3);
    return float3(h, s, l);
}

static inline __attribute__((always_inline))
float3 compositeHSL(thread const float3& destColor, thread const float3& srcColor, thread const int& op)
{
    switch (op)
    {
        case 12:
        {
            return float3(srcColor.x, destColor.y, destColor.z);
        }
        case 13:
        {
            return float3(destColor.x, srcColor.y, destColor.z);
        }
        case 14:
        {
            return float3(srcColor.x, srcColor.y, destColor.z);
        }
        default:
        {
            return float3(destColor.x, destColor.y, srcColor.z);
        }
    }
}

static inline __attribute__((always_inline))
float3 compositeHSLToRGB(thread const float3& hsl)
{
    float a = hsl.y * fast::min(hsl.z, 1.0 - hsl.z);
    float3 ks = mod(float3(0.0, 8.0, 4.0) + float3(hsl.x * 1.90985929965972900390625), float3(12.0));
    return hsl.zzz - (fast::clamp(fast::min(ks - float3(3.0), float3(9.0) - ks), float3(-1.0), float3(1.0)) * a);
}

static inline __attribute__((always_inline))
float3 compositeRGB(thread const float3& destColor, thread const float3& srcColor, thread const int& op)
{
    switch (op)
    {
        case 1:
        {
            return destColor * srcColor;
        }
        case 2:
        {
            float3 param = destColor;
            float3 param_1 = srcColor;
            return compositeScreen(param, param_1);
        }
        case 3:
        {
            float3 param_2 = srcColor;
            float3 param_3 = destColor;
            return compositeHardLight(param_2, param_3);
        }
        case 4:
        {
            return fast::min(destColor, srcColor);
        }
        case 5:
        {
            return fast::max(destColor, srcColor);
        }
        case 6:
        {
            float3 param_4 = destColor;
            float3 param_5 = srcColor;
            return compositeColorDodge(param_4, param_5);
        }
        case 7:
        {
            float3 param_6 = float3(1.0) - destColor;
            float3 param_7 = float3(1.0) - srcColor;
            return float3(1.0) - compositeColorDodge(param_6, param_7);
        }
        case 8:
        {
            float3 param_8 = destColor;
            float3 param_9 = srcColor;
            return compositeHardLight(param_8, param_9);
        }
        case 9:
        {
            float3 param_10 = destColor;
            float3 param_11 = srcColor;
            return compositeSoftLight(param_10, param_11);
        }
        case 10:
        {
            return abs(destColor - srcColor);
        }
        case 11:
        {
            return (destColor + srcColor) - ((float3(2.0) * destColor) * srcColor);
        }
        case 12:
        case 13:
        case 14:
        case 15:
        {
            float3 param_12 = destColor;
            float3 param_13 = srcColor;
            float3 param_14 = compositeRGBToHSL(param_12);
            float3 param_15 = compositeRGBToHSL(param_13);
            int param_16 = op;
            float3 param_17 = compositeHSL(param_14, param_15, param_16);
            return compositeHSLToRGB(param_17);
        }
    }
    return srcColor;
}

static inline __attribute__((always_inline))
float4 composite(thread const float4& srcColor, thread const texture2d<float> destTexture, thread const float2& destTextureSize, thread const float2& fragCoord, thread const int& op, thread sampler uSampler)
{
    if (op == 0)
    {
        return srcColor;
    }
    float2 destTexCoord = fragCoord / destTextureSize;
    float4 destColor = destTexture.sample(uSampler, destTexCoord);
    float3 param = destColor.xyz;
    float3 param_1 = srcColor.xyz;
    int param_2 = op;
    float3 blendedRGB = compositeRGB(param, param_1, param_2);
    return float4(((srcColor.xyz * (srcColor.w * (1.0 - destColor.w))) + (blendedRGB * (srcColor.w * destColor.w))) + (destColor.xyz * (1.0 - srcColor.w)), 1.0);
}

static inline __attribute__((always_inline))
void calculateColor(thread const int& tileCtrl, thread const int& ctrl, thread sampler uSampler, thread texture2d<float> uMaskTexture0, constant uMaskTextureSize0& v_1279, thread float3& vMaskTexCoord0, thread float4& vBaseColor, thread float2& vColorTexCoord0, thread texture2d<float> uColorTexture0, thread texture2d<float> uGammaLUT, constant uColorTextureSize0& v_1317, thread float4& gl_FragCoord, constant uFramebufferSize& v_1321, constant uFilterParams0& v_1324, constant uFilterParams1& v_1327, constant uFilterParams2& v_1330, thread texture2d<float> uDestTexture, thread float4& oFragColor)
{
    int maskCtrl0 = (tileCtrl >> 0) & 3;
    float maskAlpha = 1.0;
    float param = maskAlpha;
    float2 param_1 = v_1279.iMaskTextureSize0;
    float3 param_2 = vMaskTexCoord0;
    int param_3 = maskCtrl0;
    maskAlpha = sampleMask(param, uMaskTexture0, param_1, param_2, param_3, uSampler);
    float4 color = vBaseColor;
    int color0Combine = (ctrl >> 6) & 3;
    if (color0Combine != 0)
    {
        int color0Filter = (ctrl >> 4) & 3;
        float2 param_4 = vColorTexCoord0;
        float2 param_5 = v_1317.iColorTextureSize0;
        float2 param_6 = gl_FragCoord.xy;
        float2 param_7 = v_1321.iFramebufferSize;
        float4 param_8 = v_1324.iFilterParams0;
        float4 param_9 = v_1327.iFilterParams1;
        float4 param_10 = v_1330.iFilterParams2;
        int param_11 = color0Filter;
        float4 color0 = filterColor(param_4, uColorTexture0, uGammaLUT, param_5, param_6, param_7, param_8, param_9, param_10, param_11, uSampler);
        float4 param_12 = color;
        float4 param_13 = color0;
        int param_14 = color0Combine;
        color = combineColor0(param_12, param_13, param_14);
    }
    color.w *= maskAlpha;
    int compositeOp = (ctrl >> 8) & 15;
    float4 param_15 = color;
    float2 param_16 = v_1321.iFramebufferSize;
    float2 param_17 = gl_FragCoord.xy;
    int param_18 = compositeOp;
    color = composite(param_15, uDestTexture, param_16, param_17, param_18, uSampler);
    float3 _1389 = color.xyz * color.w;
    color = float4(_1389.x, _1389.y, _1389.z, color.w);
    oFragColor = color;
}

fragment main0_out main0(main0_in in [[stage_in]], constant uMaskTextureSize0& v_1279 [[buffer(0)]], constant uColorTextureSize0& v_1317 [[buffer(1)]], constant uFramebufferSize& v_1321 [[buffer(2)]], constant uFilterParams0& v_1324 [[buffer(3)]], constant uFilterParams1& v_1327 [[buffer(4)]], constant uFilterParams2& v_1330 [[buffer(5)]], constant uCtrl& _1401 [[buffer(6)]], texture2d<float> uMaskTexture0 [[texture(0)]], texture2d<float> uColorTexture0 [[texture(1)]], texture2d<float> uGammaLUT [[texture(2)]], texture2d<float> uDestTexture [[texture(3)]], sampler uSampler [[sampler(0)]], float4 gl_FragCoord [[position]])
{
    main0_out out = {};
    int param = int(in.vTileCtrl);
    int param_1 = _1401.iCtrl;
    calculateColor(param, param_1, uSampler, uMaskTexture0, v_1279, in.vMaskTexCoord0, in.vBaseColor, in.vColorTexCoord0, uColorTexture0, uGammaLUT, v_1317, gl_FragCoord, v_1321, v_1324, v_1327, v_1330, uDestTexture, out.oFragColor);
    return out;
}

