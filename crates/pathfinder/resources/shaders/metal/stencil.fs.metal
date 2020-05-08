// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct main0_out
{
    float4 oFragColor [[color(0)]];
};

fragment main0_out main0()
{
    main0_out out = {};
    out.oFragColor = float4(1.0, 0.0, 0.0, 1.0);
    return out;
}

