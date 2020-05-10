// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

struct main0_in
{
    float2 vTexCoord [[user(locn0)]];
};

fragment main0_out main0(main0_in in [[stage_in]], texture2d<float> uSrc [[texture(0)]], sampler uSampler [[sampler(0)]])
{
    main0_out out = {};
    float4 color = uSrc.sample(uSampler, in.vTexCoord);
    out.oFragColor = float4(color.xyz * color.w, color.w);
    return out;
}

