#version 450

// pathfinder/shaders/debug_texture.vs.glsl
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

uniform vec2 uFramebufferSize;
uniform vec2 uTextureSize;

in ivec2 aPosition;
in ivec2 aTexCoord;

out vec2 vTexCoord;

void main() {
    vTexCoord = vec2(aTexCoord) / uTextureSize;
    vec2 position = vec2(aPosition) / uFramebufferSize * 2.0 - 1.0;
    gl_Position = vec4(position.x, -position.y, 0.0, 1.0);
}
