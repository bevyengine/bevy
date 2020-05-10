#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!

layout(local_size_x = 16, local_size_y = 4, local_size_z = 1) in;

uniform ivec4 uFirstTileIndex[1];
layout(binding = 6, std430) restrict readonly buffer bFillTileMap
{
    int iFillTileMap[];
} _159;

layout(binding = 4, std430) restrict readonly buffer bFills
{
    uvec2 iFills[];
} _180;

layout(binding = 5, std430) restrict readonly buffer bNextFills
{
    int iNextFills[];
} _264;

layout(binding = 0) uniform writeonly image2D uDest;
layout(binding = 1) uniform sampler2D SPIRV_Cross_CombineduAreaLUTuSampler;

vec4 computeCoverage(vec2 from, vec2 to, sampler2D SPIRV_Cross_CombinedareaLUTareaLUTSampler)
{
    bvec2 _34 = bvec2(from.x < to.x);
    vec2 left = vec2(_34.x ? from.x : to.x, _34.y ? from.y : to.y);
    bvec2 _44 = bvec2(from.x < to.x);
    vec2 right = vec2(_44.x ? to.x : from.x, _44.y ? to.y : from.y);
    vec2 window = clamp(vec2(from.x, to.x), vec2(-0.5), vec2(0.5));
    float offset = mix(window.x, window.y, 0.5) - left.x;
    float t = offset / (right.x - left.x);
    float y = mix(left.y, right.y, t);
    float d = (right.y - left.y) / (right.x - left.x);
    float dX = window.x - window.y;
    return textureLod(SPIRV_Cross_CombinedareaLUTareaLUTSampler, vec2(y + 8.0, abs(d * dX)) / vec2(16.0), 0.0) * dX;
}

void main()
{
    ivec2 tileSubCoord = ivec2(gl_LocalInvocationID.xy) * ivec2(1, 4);
    uint tileIndexOffset = gl_WorkGroupID.z;
    uint tileIndex = tileIndexOffset + uint(uFirstTileIndex[0].x);
    int fillIndex = _159.iFillTileMap[tileIndex];
    if (fillIndex < 0)
    {
        return;
    }
    vec4 coverages = vec4(0.0);
    do
    {
        uvec2 fill = _180.iFills[fillIndex];
        vec2 from = vec2(float(fill.y & 15u), float((fill.y >> 4u) & 15u)) + (vec2(float(fill.x & 255u), float((fill.x >> 8u) & 255u)) / vec2(256.0));
        vec2 to = vec2(float((fill.y >> 8u) & 15u), float((fill.y >> 12u) & 15u)) + (vec2(float((fill.x >> 16u) & 255u), float((fill.x >> 24u) & 255u)) / vec2(256.0));
        vec2 param = from - (vec2(tileSubCoord) + vec2(0.5));
        vec2 param_1 = to - (vec2(tileSubCoord) + vec2(0.5));
        coverages += computeCoverage(param, param_1, SPIRV_Cross_CombineduAreaLUTuSampler);
        fillIndex = _264.iNextFills[fillIndex];
    } while (fillIndex >= 0);
    ivec2 tileOrigin = ivec2(int(tileIndex & 255u), int((tileIndex >> 8u) & 255u)) * ivec2(16, 4);
    ivec2 destCoord = tileOrigin + ivec2(gl_LocalInvocationID.xy);
    imageStore(uDest, destCoord, coverages);
}

