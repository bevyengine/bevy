#version 430

// pathfinder/shaders/fill.cs.glsl
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#extension GL_GOOGLE_include_directive : enable

precision highp float;

#ifdef GL_ES
precision highp sampler2D;
#endif

#include "fill.inc.glsl"

layout(local_size_x = 16, local_size_y = 4) in;

layout (set=0, binding = 0) uniform writeonly image2D uDest;
layout (set=0, binding = 1) uniform texture2D uAreaLUT;
layout (set=0, binding = 2) uniform sampler uSampler;
layout (set=0, binding = 3) uniform uFirstTileIndex {
    int firstTileIndex;
};

layout(std430, set=0, binding = 4) buffer bFills {
    restrict readonly uvec2 iFills[];
};

layout(std430, set=0, binding = 5) buffer bNextFills {
    restrict readonly int iNextFills[];
};

layout(std430, set=0, binding = 6) buffer bFillTileMap {
    restrict readonly int iFillTileMap[];
};

void main() {
    ivec2 tileSubCoord = ivec2(gl_LocalInvocationID.xy) * ivec2(1, 4);
    uint tileIndexOffset = gl_WorkGroupID.z;

    uint tileIndex = tileIndexOffset + uint(firstTileIndex);

    int fillIndex = iFillTileMap[tileIndex];
    if (fillIndex < 0)
        return;

    vec4 coverages = vec4(0.0);
    do {
        uvec2 fill = iFills[fillIndex];
        vec2 from = vec2(fill.y & 0xf,           (fill.y >> 4u) & 0xf) +
                    vec2(fill.x & 0xff,          (fill.x >> 8u) & 0xff) / 256.0;
        vec2 to   = vec2((fill.y >> 8u) & 0xf,   (fill.y >> 12u) & 0xf) +
                    vec2((fill.x >> 16u) & 0xff, (fill.x >> 24u) & 0xff) / 256.0;

        coverages += computeCoverage(from - (vec2(tileSubCoord) + vec2(0.5)),
                                     to   - (vec2(tileSubCoord) + vec2(0.5)),
                                     uAreaLUT, uSampler);

        fillIndex = iNextFills[fillIndex];
    } while (fillIndex >= 0);

    ivec2 tileOrigin = ivec2(tileIndex & 0xff, (tileIndex >> 8u) & 0xff) * ivec2(16, 4);
    ivec2 destCoord = tileOrigin + ivec2(gl_LocalInvocationID.xy);
    imageStore(uDest, destCoord, coverages);
}
