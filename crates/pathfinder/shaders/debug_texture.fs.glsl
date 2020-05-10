#version 450

// pathfinder/shaders/debug_texture.fs.glsl
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

layout(set=0, binding=2) uniform texture2D uTexture;
layout(set=0, binding=3) uniform sampler uSampler;
layout(set=0, binding=4) uniform uColor {
    vec4 color;
};

in vec2 vTexCoord;

out vec4 oFragColor;

void main() {
    float alpha = texture(sampler2D(uTexture, uSampler), vTexCoord).r * color.a;
    oFragColor = alpha * vec4(color.rgb, 1.0);
}
