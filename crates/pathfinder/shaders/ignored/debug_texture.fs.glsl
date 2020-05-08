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
precision highp sampler2D;

uniform sampler2D uTexture;
uniform vec4 uColor;

in vec2 vTexCoord;

out vec4 oFragColor;

void main() {
    float alpha = texture(uTexture, vTexCoord).r * uColor.a;
    oFragColor = alpha * vec4(uColor.rgb, 1.0);
}
