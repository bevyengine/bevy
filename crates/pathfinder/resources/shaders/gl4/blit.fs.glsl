#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


layout(binding = 0) uniform sampler2D SPIRV_Cross_CombineduSrcuSampler;

layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 oFragColor;

void main()
{
    vec4 color = texture(SPIRV_Cross_CombineduSrcuSampler, vTexCoord);
    oFragColor = vec4(color.xyz * color.w, color.w);
}

