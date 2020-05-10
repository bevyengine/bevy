#version 450

// pathfinder/shaders/tile_copy.fs.glsl
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

layout(set=0, binding=2) uniform uFramebufferSize {
    vec2 framebufferSize;
};
layout(set=0, binding=3) uniform texture2D uSrc;
layout(set=0, binding=4) uniform sampler uSampler;

out vec4 oFragColor;

void main() {
    vec2 texCoord = gl_FragCoord.xy / framebufferSize;
    oFragColor = texture(sampler2D(uSrc, uSampler), texCoord);
}
