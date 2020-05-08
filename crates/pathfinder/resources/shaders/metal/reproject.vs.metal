// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    constant float4x4* uNewTransform [[id(0)]];
};

struct main0_out
{
    float2 vTexCoord [[user(locn0)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    int2 aPosition [[attribute(0)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    float2 position = float2(in.aPosition);
    out.vTexCoord = position;
    position.y = 1.0 - position.y;
    out.gl_Position = (*spvDescriptorSet0.uNewTransform) * float4(position, 0.0, 1.0);
    return out;
}

