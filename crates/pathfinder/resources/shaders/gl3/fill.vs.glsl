#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;

uniform uFramebufferSize {
    vec2 framebufferSize;
};
uniform vec2 uTileSize;

in uvec2 aTessCoord;
in uint aFromPx;
in uint aToPx;
in vec2 aFromSubpx;
in vec2 aToSubpx;
in uint aTileIndex;

out vec2 vFrom;
out vec2 vTo;

vec2 computeTileOffset(uint tileIndex, float stencilTextureWidth){
    uint tilesPerRow = uint(stencilTextureWidth / uTileSize . x);
    uvec2 tileOffset = uvec2(tileIndex % tilesPerRow, tileIndex / tilesPerRow);
    return vec2(tileOffset)* uTileSize;
}

void main(){
    vec2 tileOrigin = computeTileOffset(aTileIndex, framebufferSize . x);

    vec2 from = vec2(aFromPx & 15u, aFromPx >> 4u)+ aFromSubpx;
    vec2 to = vec2(aToPx & 15u, aToPx >> 4u)+ aToSubpx;

    vec2 position;
    if(aTessCoord . x == 0u)
        position . x = floor(min(from . x, to . x));
    else
        position . x = ceil(max(from . x, to . x));
    if(aTessCoord . y == 0u)
        position . y = floor(min(from . y, to . y));
    else
        position . y = uTileSize . y;

    vFrom = from - position;
    vTo = to - position;

    vec2 globalPosition =(tileOrigin + position)/ framebufferSize * 2.0 - 1.0;



    gl_Position = vec4(globalPosition, 0.0, 1.0);
}

