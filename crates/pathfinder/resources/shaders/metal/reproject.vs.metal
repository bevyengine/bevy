// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct uNewTransform
{
    float4x4 newTransform;
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

vertex main0_out main0(main0_in in [[stage_in]], constant uNewTransform& _36 [[buffer(0)]])
{
    main0_out out = {};
    float2 position = float2(in.aPosition);
    out.vTexCoord = position;
    position.y = 1.0 - position.y;
    out.gl_Position = _36.newTransform * float4(position, 0.0, 1.0);
    return out;
}

