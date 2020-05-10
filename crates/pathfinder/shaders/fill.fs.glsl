#version 450

// pathfinder/shaders/fill.fs.glsl
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

layout(set=0, binding=2) uniform texture2D uAreaLUT;
layout(set=0, binding=3) uniform sampler uSampler;

in vec2 vFrom;
in vec2 vTo;

out vec4 oFragColor;

void main() {
    oFragColor = computeCoverage(vFrom, vTo, uAreaLUT, uSampler);
}
