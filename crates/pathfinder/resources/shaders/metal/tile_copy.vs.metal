// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    constant float2* uTileSize [[id(0)]];
    constant float4x4* uTransform [[id(1)]];
};

struct main0_out
{
    float4 gl_Position [[position]];
};

struct main0_in
{
    int2 aTilePosition [[attribute(0)]];
};

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    float2 position = float2(in.aTilePosition) * (*spvDescriptorSet0.uTileSize);
    out.gl_Position = (*spvDescriptorSet0.uTransform) * float4(position, 0.0, 1.0);
    return out;
}

