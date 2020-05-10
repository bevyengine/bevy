#version 450

// pathfinder/shaders/tile_clip.vs.glsl
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

in ivec2 aTileOffset;
in ivec2 aDestTileOrigin;
in ivec2 aSrcTileOrigin;
in int aSrcBackdrop;

out vec2 vTexCoord;
out float vBackdrop;

void main() {
    vec2 destPosition = vec2(aDestTileOrigin + aTileOffset) / vec2(256.0);
    vec2 srcPosition = vec2(aSrcTileOrigin + aTileOffset) / vec2(256.0);
    vTexCoord = srcPosition;
    vBackdrop = float(aSrcBackdrop);
    gl_Position = vec4(mix(vec2(-1.0), vec2(1.0), destPosition), 0.0, 1.0);
}
