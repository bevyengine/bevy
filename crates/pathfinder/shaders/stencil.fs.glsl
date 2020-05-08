#version 450

// pathfinder/shaders/stencil.fs.glsl
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

precision highp float;
precision highp sampler2D;

out vec4 oFragColor;

void main() {
    // This should be color masked out.
    oFragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
