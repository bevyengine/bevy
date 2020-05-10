#version 450

// pathfinder/shaders/tile_clip.fs.glsl
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

layout(set=0, binding=0) uniform texture2D uSrc;
layout(set=0, binding=1) uniform sampler uSampler;

in vec2 vTexCoord;
in float vBackdrop;

out vec4 oFragColor;

void main() {
    oFragColor = clamp(abs(texture(sampler2D(uSrc, uSampler), vTexCoord) + vBackdrop), 0.0, 1.0);
}
