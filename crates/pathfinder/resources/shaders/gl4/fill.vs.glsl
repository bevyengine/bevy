#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uTileSize[1];
uniform vec4 uFramebufferSize[1];
layout(location = 5) in uint aTileIndex;
layout(location = 3) in uint aFromPx;
layout(location = 1) in vec2 aFromSubpx;
layout(location = 4) in uint aToPx;
layout(location = 2) in vec2 aToSubpx;
layout(location = 0) in uvec2 aTessCoord;
layout(location = 0) out vec2 vFrom;
layout(location = 1) out vec2 vTo;

vec2 computeTileOffset(uint tileIndex, float stencilTextureWidth)
{
    uint tilesPerRow = uint(stencilTextureWidth / uTileSize[0].x);
    uvec2 tileOffset = uvec2(tileIndex % tilesPerRow, tileIndex / tilesPerRow);
    return (vec2(tileOffset) * uTileSize[0].xy) * vec2(1.0, 0.25);
}

void main()
{
    uint param = aTileIndex;
    float param_1 = uFramebufferSize[0].x;
    vec2 tileOrigin = computeTileOffset(param, param_1);
    vec2 from = vec2(float(aFromPx & 15u), float(aFromPx >> 4u)) + aFromSubpx;
    vec2 to = vec2(float(aToPx & 15u), float(aToPx >> 4u)) + aToSubpx;
    vec2 position;
    if (aTessCoord.x == 0u)
    {
        position.x = floor(min(from.x, to.x));
    }
    else
    {
        position.x = ceil(max(from.x, to.x));
    }
    if (aTessCoord.y == 0u)
    {
        position.y = floor(min(from.y, to.y));
    }
    else
    {
        position.y = uTileSize[0].y;
    }
    position.y = floor(position.y * 0.25);
    vec2 offset = vec2(0.0, 1.5) - (position * vec2(1.0, 4.0));
    vFrom = from + offset;
    vTo = to + offset;
    vec2 globalPosition = (((tileOrigin + position) / uFramebufferSize[0].xy) * 2.0) - vec2(1.0);
    gl_Position = vec4(globalPosition, 0.0, 1.0);
}

