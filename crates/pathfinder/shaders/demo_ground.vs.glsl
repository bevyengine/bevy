#version 450

// pathfinder/resources/shaders/demo_ground.vs.glsl
//
// Copyright © 2019 The Pathfinder Project Developers.
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

layout(set=0, binding=0) uniform uTransform {
    mat4 transform;
};
layout(set=0, binding=1) uniform uGridlineCount {
    int gridlineCount;
};

in ivec2 aPosition;

out vec2 vTexCoord;

void main() {
    vTexCoord = vec2(aPosition * gridlineCount);
    gl_Position = transform * vec4(ivec4(aPosition.x, 0, aPosition.y, 1));
}
