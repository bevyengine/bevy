// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    texture2d<float> uTexture [[id(0)]];
    sampler uTextureSmplr [[id(1)]];
    constant float4* uColor [[id(2)]];
};

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

struct main0_in
{
    float2 vTexCoord [[user(locn0)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    float alpha = spvDescriptorSet0.uTexture.sample(spvDescriptorSet0.uTextureSmplr, in.vTexCoord).x * (*spvDescriptorSet0.uColor).w;
    out.oFragColor = float4((*spvDescriptorSet0.uColor).xyz, 1.0) * alpha;
    return out;
}

