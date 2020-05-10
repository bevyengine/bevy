#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uTileSize[1];
uniform ivec4 uTextureMetadataSize[1];
uniform vec4 uTransform[4];
layout(binding = 2) uniform sampler2D SPIRV_Cross_CombineduTextureMetadatauSampler;

layout(location = 1) in ivec2 aTileOrigin;
layout(location = 0) in ivec2 aTileOffset;
layout(location = 2) in uvec2 aMaskTexCoord0;
layout(location = 4) in int aColor;
layout(location = 1) out vec2 vColorTexCoord0;
layout(location = 0) out vec3 vMaskTexCoord0;
layout(location = 3) in ivec2 aMaskBackdrop;
layout(location = 2) out vec4 vBaseColor;
layout(location = 3) out float vTileCtrl;
layout(location = 5) in int aTileCtrl;

void main()
{
    vec2 tileOrigin = vec2(aTileOrigin);
    vec2 tileOffset = vec2(aTileOffset);
    vec2 position = (tileOrigin + tileOffset) * uTileSize[0].xy;
    vec2 maskTexCoord0 = (vec2(aMaskTexCoord0) + tileOffset) * uTileSize[0].xy;
    vec2 textureMetadataScale = vec2(1.0) / vec2(uTextureMetadataSize[0].xy);
    vec2 metadataEntryCoord = vec2(float((aColor % 128) * 4), float(aColor / 128));
    vec2 colorTexMatrix0Coord = (metadataEntryCoord + vec2(0.5)) * textureMetadataScale;
    vec2 colorTexOffsetsCoord = (metadataEntryCoord + vec2(1.5, 0.5)) * textureMetadataScale;
    vec2 baseColorCoord = (metadataEntryCoord + vec2(2.5, 0.5)) * textureMetadataScale;
    vec4 colorTexMatrix0 = textureLod(SPIRV_Cross_CombineduTextureMetadatauSampler, colorTexMatrix0Coord, 0.0);
    vec4 colorTexOffsets = textureLod(SPIRV_Cross_CombineduTextureMetadatauSampler, colorTexOffsetsCoord, 0.0);
    vec4 baseColor = textureLod(SPIRV_Cross_CombineduTextureMetadatauSampler, baseColorCoord, 0.0);
    vColorTexCoord0 = (mat2(vec2(colorTexMatrix0.xy), vec2(colorTexMatrix0.zw)) * position) + colorTexOffsets.xy;
    vMaskTexCoord0 = vec3(maskTexCoord0, float(aMaskBackdrop.x));
    vBaseColor = baseColor;
    vTileCtrl = float(aTileCtrl);
    gl_Position = mat4(uTransform[0], uTransform[1], uTransform[2], uTransform[3]) * vec4(position, 0.0, 1.0);
}

