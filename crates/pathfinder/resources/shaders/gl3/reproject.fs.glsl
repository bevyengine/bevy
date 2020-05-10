#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uOldTransform[4];
uniform sampler2D SPIRV_Cross_CombineduTextureuSampler;

in vec2 vTexCoord;
layout(location = 0) out vec4 oFragColor;

void main()
{
    vec4 normTexCoord = mat4(uOldTransform[0], uOldTransform[1], uOldTransform[2], uOldTransform[3]) * vec4(vTexCoord, 0.0, 1.0);
    vec2 texCoord = ((normTexCoord.xy / vec2(normTexCoord.w)) + vec2(1.0)) * 0.5;
    oFragColor = texture(SPIRV_Cross_CombineduTextureuSampler, texCoord);
}

