// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    texture2d<float> uSrc [[id(0)]];
    sampler uSrcSmplr [[id(1)]];
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
    float4 color = spvDescriptorSet0.uSrc.sample(spvDescriptorSet0.uSrcSmplr, in.vTexCoord);
    out.oFragColor = float4(color.xyz * color.w, color.w);
    return out;
}

