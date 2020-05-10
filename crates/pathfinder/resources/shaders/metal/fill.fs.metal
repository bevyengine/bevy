// Automatically generated from files in pathfinder/shaders/. Do not edit!
#pragma clang diagnostic ignored "-Wmissing-prototypes"

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

struct main0_in
{
    float2 vFrom [[user(locn0)]];
    float2 vTo [[user(locn1)]];
};

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
    return areaLUT.sample(areaLUTSampler, (float2(y + 8.0, abs(d * dX)) / float2(16.0))) * dX;
}

fragment main0_out main0(main0_in in [[stage_in]], texture2d<float> uAreaLUT [[texture(0)]], sampler uSampler [[sampler(0)]])
{
    main0_out out = {};
    float2 param = in.vFrom;
    float2 param_1 = in.vTo;
    out.oFragColor = computeCoverage(param, param_1, uAreaLUT, uSampler);
    return out;
}

