// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct uFramebufferSize
{
    float2 framebufferSize;
};

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

fragment main0_out main0(constant uFramebufferSize& _17 [[buffer(0)]], texture2d<float> uSrc [[texture(0)]], sampler uSampler [[sampler(0)]], float4 gl_FragCoord [[position]])
{
    main0_out out = {};
    float2 texCoord = gl_FragCoord.xy / _17.framebufferSize;
    out.oFragColor = uSrc.sample(uSampler, texCoord);
    return out;
}

