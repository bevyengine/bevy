#version 450

// pathfinder/shaders/blit.fs.glsl
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

layout(set=0, binding=0) uniform texture2D uSrc;
layout(set=0, binding=1) uniform sampler uSrcSampler;

in vec2 vTexCoord;

out vec4 oFragColor;

void main() {
    vec4 color = texture(sampler2D(uSrc, uSrcSampler), vTexCoord);
    oFragColor = vec4(color.rgb * color.a, color.a);
}
