#version 450

// pathfinder/shaders/tile_copy.vs.glsl
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

precision highp float;
precision highp sampler2D;

layout(set=0, binding=0) uniform uTransform {
    mat4 transform;
};
layout(set=0, binding=1) uniform uTileSize {
    vec2 tileSize;
};

in ivec2 aTilePosition;

void main() {
    vec2 position = vec2(aTilePosition) * tileSize;
    gl_Position = transform * vec4(position, 0.0, 1.0);
}
