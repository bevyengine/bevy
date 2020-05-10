#version 450

// pathfinder/shaders/tile.vs.glsl
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

layout(set=0, binding=0) uniform uTransform {
    mat4 transform; 
};
layout(set=0, binding=1) uniform uTileSize {
    vec2 tileSize; 
};
layout(set=0, binding=2) uniform texture2D uTextureMetadata;
layout(set=0, binding=3) uniform sampler uSampler;
layout(set=0, binding=4) uniform uTextureMetadataSize {
    ivec2 textureMetadataSize; 
};

in ivec2 aTileOffset;
in ivec2 aTileOrigin;
in uvec2 aMaskTexCoord0;
in ivec2 aMaskBackdrop;
in int aColor;
in int aTileCtrl;

out vec3 vMaskTexCoord0;
out vec2 vColorTexCoord0;
out vec4 vBaseColor;
out float vTileCtrl;

void main() {
    vec2 tileOrigin = vec2(aTileOrigin), tileOffset = vec2(aTileOffset);
    vec2 position = (tileOrigin + tileOffset) * tileSize;

    vec2 maskTexCoord0 = (vec2(aMaskTexCoord0) + tileOffset) * tileSize;

    vec2 textureMetadataScale = vec2(1.0) / vec2(textureMetadataSize);
    vec2 metadataEntryCoord = vec2(aColor % 128 * 4, aColor / 128);
    vec2 colorTexMatrix0Coord = (metadataEntryCoord + vec2(0.5, 0.5)) * textureMetadataScale;
    vec2 colorTexOffsetsCoord = (metadataEntryCoord + vec2(1.5, 0.5)) * textureMetadataScale;
    vec2 baseColorCoord = (metadataEntryCoord + vec2(2.5, 0.5)) * textureMetadataScale;
    vec4 colorTexMatrix0 = texture(sampler2D(uTextureMetadata, uSampler), colorTexMatrix0Coord);
    vec4 colorTexOffsets = texture(sampler2D(uTextureMetadata, uSampler), colorTexOffsetsCoord);
    vec4 baseColor = texture(sampler2D(uTextureMetadata, uSampler), baseColorCoord);

    vColorTexCoord0 = mat2(colorTexMatrix0) * position + colorTexOffsets.xy;
    vMaskTexCoord0 = vec3(maskTexCoord0, float(aMaskBackdrop.x));
    vBaseColor = baseColor;
    vTileCtrl = float(aTileCtrl);
    gl_Position = transform * vec4(position, 0.0, 1.0);
}
