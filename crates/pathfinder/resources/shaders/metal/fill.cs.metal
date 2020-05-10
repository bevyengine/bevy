// Automatically generated from files in pathfinder/shaders/. Do not edit!
#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct uFirstTileIndex
{
    int firstTileIndex;
};

struct bFillTileMap
{
    int iFillTileMap[1];
};

struct bFills
{
    uint2 iFills[1];
};

struct bNextFills
{
    int iNextFills[1];
};

constant uint3 gl_WorkGroupSize [[maybe_unused]] = uint3(16u, 4u, 1u);

static inline __attribute__((always_inline))
float4 computeCoverage(thread const float2& from, thread const float2& to, thread const texture2d<float> areaLUT, thread const sampler areaLUTSampler)
{
    float2 left = select(to, from, bool2(from.x < to.x));
    float2 right = select(from, to, bool2(from.x < to.x));
    float2 window = fast::clamp(float2(from.x, to.x), float2(-0.5), float2(0.5));
    float offset = mix(window.x, window.y, 0.5) - left.x;
    float t = offset / (right.x - left.x);
    float y = mix(left.y, right.y, t);
    float d = (right.y - left.y) / (right.x - left.x);
    float dX = window.x - window.y;
    return areaLUT.sample(areaLUTSampler, (float2(y + 8.0, abs(d * dX)) / float2(16.0)), level(0.0)) * dX;
}

kernel void main0(constant uFirstTileIndex& _147 [[buffer(0)]], const device bFillTileMap& _159 [[buffer(1)]], const device bFills& _180 [[buffer(2)]], const device bNextFills& _264 [[buffer(3)]], texture2d<float> uAreaLUT [[texture(0)]], texture2d<float, access::write> uDest [[texture(1)]], sampler uSampler [[sampler(0)]], uint3 gl_LocalInvocationID [[thread_position_in_threadgroup]], uint3 gl_WorkGroupID [[threadgroup_position_in_grid]])
{
    int2 tileSubCoord = int2(gl_LocalInvocationID.xy) * int2(1, 4);
    uint tileIndexOffset = gl_WorkGroupID.z;
    uint tileIndex = tileIndexOffset + uint(_147.firstTileIndex);
    int fillIndex = _159.iFillTileMap[tileIndex];
    if (fillIndex < 0)
    {
        return;
    }
    float4 coverages = float4(0.0);
    do
    {
        uint2 fill = _180.iFills[fillIndex];
        float2 from = float2(float(fill.y & 15u), float((fill.y >> 4u) & 15u)) + (float2(float(fill.x & 255u), float((fill.x >> 8u) & 255u)) / float2(256.0));
        float2 to = float2(float((fill.y >> 8u) & 15u), float((fill.y >> 12u) & 15u)) + (float2(float((fill.x >> 16u) & 255u), float((fill.x >> 24u) & 255u)) / float2(256.0));
        float2 param = from - (float2(tileSubCoord) + float2(0.5));
        float2 param_1 = to - (float2(tileSubCoord) + float2(0.5));
        coverages += computeCoverage(param, param_1, uAreaLUT, uSampler);
        fillIndex = _264.iNextFills[fillIndex];
    } while (fillIndex >= 0);
    int2 tileOrigin = int2(int(tileIndex & 255u), int((tileIndex >> 8u) & 255u)) * int2(16, 4);
    int2 destCoord = tileOrigin + int2(gl_LocalInvocationID.xy);
    uDest.write(coverages, uint2(destCoord));
}

