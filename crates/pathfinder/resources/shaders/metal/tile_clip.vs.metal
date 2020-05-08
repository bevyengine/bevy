// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct main0_out
{
    float2 vTexCoord [[user(locn0)]];
    float vBackdrop [[user(locn1)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    int2 aTileOffset [[attribute(0)]];
    int2 aDestTileOrigin [[attribute(1)]];
    int2 aSrcTileOrigin [[attribute(2)]];
    int aSrcBackdrop [[attribute(3)]];
};

vertex main0_out main0(main0_in in [[stage_in]])
{
    main0_out out = {};
    float2 destPosition = float2(in.aDestTileOrigin + in.aTileOffset) / float2(256.0);
    float2 srcPosition = float2(in.aSrcTileOrigin + in.aTileOffset) / float2(256.0);
    out.vTexCoord = srcPosition;
    out.vBackdrop = float(in.aSrcBackdrop);
    out.gl_Position = float4(mix(float2(-1.0), float2(1.0), destPosition), 0.0, 1.0);
    return out;
}

