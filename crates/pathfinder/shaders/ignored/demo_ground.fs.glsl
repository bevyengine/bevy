#version 450

// pathfinder/resources/shaders/demo_ground.fs.glsl
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

uniform vec4 uGroundColor;
uniform vec4 uGridlineColor;

in vec2 vTexCoord;

out vec4 oFragColor;

void main() {
    vec2 texCoordPx = fract(vTexCoord) / fwidth(vTexCoord);
    oFragColor = any(lessThanEqual(texCoordPx, vec2(1.0))) ? uGridlineColor : uGroundColor;
}
