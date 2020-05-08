#version 450

// pathfinder/shaders/fill.vs.glsl
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

precision highp float;
precision highp sampler2D;

layout(set=0, binding=0) uniform uFramebufferSize {
    vec2 framebufferSize;
};
layout(set=0, binding=1) uniform uTileSize {
    vec2 tileSize;
};

in uvec2 aTessCoord;
in uint aFromPx;
in uint aToPx;
in vec2 aFromSubpx;
in vec2 aToSubpx;
in uint aTileIndex;

out vec2 vFrom;
out vec2 vTo;

vec2 computeTileOffset(uint tileIndex, float stencilTextureWidth) {
    uint tilesPerRow = uint(stencilTextureWidth / tileSize.x);
    uvec2 tileOffset = uvec2(tileIndex % tilesPerRow, tileIndex / tilesPerRow);
    return vec2(tileOffset) * tileSize;
}

void main() {
    vec2 tileOrigin = computeTileOffset(aTileIndex, framebufferSize.x);

    vec2 from = vec2(aFromPx & 15u, aFromPx >> 4u) + aFromSubpx;
    vec2 to = vec2(aToPx & 15u, aToPx >> 4u) + aToSubpx;

    vec2 position;
    if (aTessCoord.x == 0u)
        position.x = floor(min(from.x, to.x));
    else
        position.x = ceil(max(from.x, to.x));
    if (aTessCoord.y == 0u)
        position.y = floor(min(from.y, to.y));
    else
        position.y = tileSize.y;

    vFrom = from - position;
    vTo = to - position;

    vec2 globalPosition = (tileOrigin + position) / framebufferSize * 2.0 - 1.0;
#ifdef PF_ORIGIN_UPPER_LEFT
    globalPosition.y = -globalPosition.y;
#endif
    gl_Position = vec4(globalPosition, 0.0, 1.0);
}
