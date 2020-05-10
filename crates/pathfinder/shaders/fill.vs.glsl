#version 450

// pathfinder/shaders/fill.vs.glsl
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

precision highp float;

#ifdef GL_ES
precision highp sampler2D;
#endif

layout(std140, set=0, binding=0) uniform uFramebufferSize {
    vec2 framebufferSize;
};
layout(std140, set=0, binding=1) uniform uTileSize {
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
    return vec2(tileOffset) * tileSize * vec2(1.0, 0.25);
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
    position.y = floor(position.y * 0.25);

    // Since each fragment corresponds to 4 pixels on a scanline, the varying interpolation will
    // land the fragment halfway between the four-pixel strip, at pixel offset 2.0. But we want to
    // do our coverage calculation on the center of the first pixel in the strip instead, at pixel
    // offset 0.5. This adjustment of 1.5 accomplishes that.
    vec2 offset = vec2(0.0, 1.5) - position * vec2(1.0, 4.0);
    vFrom = from + offset;
    vTo = to + offset;

    vec2 globalPosition = (tileOrigin + position) / framebufferSize * 2.0 - 1.0;
#ifdef PF_ORIGIN_UPPER_LEFT
    globalPosition.y = -globalPosition.y;
#endif
    gl_Position = vec4(globalPosition, 0.0, 1.0);
}
