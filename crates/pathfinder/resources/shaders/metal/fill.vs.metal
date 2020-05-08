// Automatically generated from files in pathfinder/shaders/. Do not edit!
#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct spvDescriptorSetBuffer0
{
    constant float2* uTileSize [[id(0)]];
    constant float2* uFramebufferSize [[id(1)]];
};

struct main0_out
{
    float2 vFrom [[user(locn0)]];
    float2 vTo [[user(locn1)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    uint2 aTessCoord [[attribute(0)]];
    uint aFromPx [[attribute(1)]];
    uint aToPx [[attribute(2)]];
    float2 aFromSubpx [[attribute(3)]];
    float2 aToSubpx [[attribute(4)]];
    uint aTileIndex [[attribute(5)]];
};

static inline __attribute__((always_inline))
float2 computeTileOffset(thread const uint& tileIndex, thread const float& stencilTextureWidth, thread float2 uTileSize)
{
    uint tilesPerRow = uint(stencilTextureWidth / uTileSize.x);
    uint2 tileOffset = uint2(tileIndex % tilesPerRow, tileIndex / tilesPerRow);
    return float2(tileOffset) * uTileSize;
}

vertex main0_out main0(main0_in in [[stage_in]], constant spvDescriptorSetBuffer0& spvDescriptorSet0 [[buffer(0)]])
{
    main0_out out = {};
    uint param = in.aTileIndex;
    float param_1 = (*spvDescriptorSet0.uFramebufferSize).x;
    float2 tileOrigin = computeTileOffset(param, param_1, (*spvDescriptorSet0.uTileSize));
    float2 from = float2(float(in.aFromPx & 15u), float(in.aFromPx >> 4u)) + in.aFromSubpx;
    float2 to = float2(float(in.aToPx & 15u), float(in.aToPx >> 4u)) + in.aToSubpx;
    float2 position;
    if (in.aTessCoord.x == 0u)
    {
        position.x = floor(fast::min(from.x, to.x));
    }
    else
    {
        position.x = ceil(fast::max(from.x, to.x));
    }
    if (in.aTessCoord.y == 0u)
    {
        position.y = floor(fast::min(from.y, to.y));
    }
    else
    {
        position.y = (*spvDescriptorSet0.uTileSize).y;
    }
    out.vFrom = from - position;
    out.vTo = to - position;
    float2 globalPosition = (((tileOrigin + position) / (*spvDescriptorSet0.uFramebufferSize)) * 2.0) - float2(1.0);
    globalPosition.y = -globalPosition.y;
    out.gl_Position = float4(globalPosition, 0.0, 1.0);
    return out;
}

