// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct uGridlineColor
{
    float4 gridlineColor;
};

struct uGroundColor
{
    float4 groundColor;
};

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

struct main0_in
{
    float2 vTexCoord [[user(locn0)]];
};

fragment main0_out main0(main0_in in [[stage_in]], constant uGridlineColor& _33 [[buffer(0)]], constant uGroundColor& _42 [[buffer(1)]])
{
    main0_out out = {};
    float2 texCoordPx = fract(in.vTexCoord) / fwidth(in.vTexCoord);
    float4 _28;
    if (any(texCoordPx <= float2(1.0)))
    {
        _28 = _33.gridlineColor;
    }
    else
    {
        _28 = _42.groundColor;
    }
    out.oFragColor = _28;
    return out;
}

