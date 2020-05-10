#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


layout(binding = 0) uniform sampler2D SPIRV_Cross_CombineduSrcuSampler;

layout(location = 0) out vec4 oFragColor;
layout(location = 0) in vec2 vTexCoord;
layout(location = 1) in float vBackdrop;

void main()
{
    oFragColor = clamp(abs(texture(SPIRV_Cross_CombineduSrcuSampler, vTexCoord) + vec4(vBackdrop)), vec4(0.0), vec4(1.0));
}

