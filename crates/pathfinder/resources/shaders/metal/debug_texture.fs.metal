// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct uColor
{
    float4 color;
};

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

struct main0_in
{
    float2 vTexCoord [[user(locn0)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant uColor& _30 [[buffer(0)]], texture2d<float> uTexture [[texture(0)]], sampler uSampler [[sampler(0)]])
{
    main0_out out = {};
    float alpha = uTexture.sample(uSampler, in.vTexCoord).x * _30.color.w;
    out.oFragColor = float4(_30.color.xyz, 1.0) * alpha;
    return out;
}

