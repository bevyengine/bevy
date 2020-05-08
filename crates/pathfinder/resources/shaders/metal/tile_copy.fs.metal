// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    constant float2* uFramebufferSize [[id(0)]];
    texture2d<float> uSrc [[id(1)]];
    sampler uSrcSmplr [[id(2)]];
};

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

fragment main0_out main0(constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]], float4 gl_FragCoord [[position]])
{
    main0_out out = {};
    float2 texCoord = gl_FragCoord.xy / (*spvDescriptorSet0.uFramebufferSize);
    out.oFragColor = spvDescriptorSet0.uSrc.sample(spvDescriptorSet0.uSrcSmplr, texCoord);
    return out;
}

