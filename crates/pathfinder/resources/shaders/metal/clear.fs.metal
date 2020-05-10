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

fragment main0_out main0(constant uColor& _12 [[buffer(0)]])
{
    main0_out out = {};
    out.oFragColor = float4(_12.color.xyz, 1.0) * _12.color.w;
    return out;
}

