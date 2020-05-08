// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    constant float4* uColor [[id(0)]];
};

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

fragment main0_out main0(constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    out.oFragColor = float4((*spvDescriptorSet0.uColor).xyz, 1.0) * (*spvDescriptorSet0.uColor).w;
    return out;
}

