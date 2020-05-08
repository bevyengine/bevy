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
    float vBackdrop [[user(locn1)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    float4 alpha_texture_color = spvDescriptorSet0.uSrc.sample(spvDescriptorSet0.uSrcSmplr, in.vTexCoord);
    float alpha = fast::clamp(abs(alpha_texture_color.x + in.vBackdrop), 0.0, 1.0);
    out.oFragColor = float4(alpha, 0.0, 0.0, 1.0);
    return out;
}

