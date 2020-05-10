#version 450

// pathfinder/shaders/debug_solid.fs.glsl
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

layout(set=0, binding=1) uniform uColor {
    vec4 color;
};

out vec4 oFragColor;

void main() {
    oFragColor = vec4(color.rgb, 1.0) * color.a;
}
