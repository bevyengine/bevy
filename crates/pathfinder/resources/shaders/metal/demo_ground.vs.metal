// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct uGridlineCount
{
    int gridlineCount;
};

struct uTransform
{
    float4x4 transform;
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

vertex main0_out main0(main0_in in [[stage_in]], constant uGridlineCount& _17 [[buffer(0)]], constant uTransform& _35 [[buffer(1)]])
{
    main0_out out = {};
    out.vTexCoord = float2(in.aPosition * int2(_17.gridlineCount));
    out.gl_Position = _35.transform * float4(int4(in.aPosition.x, 0, in.aPosition.y, 1));
    return out;
}

