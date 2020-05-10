#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uColor[1];
uniform sampler2D SPIRV_Cross_CombineduTextureuSampler;

in vec2 vTexCoord;
layout(location = 0) out vec4 oFragColor;

void main()
{
    float alpha = texture(SPIRV_Cross_CombineduTextureuSampler, vTexCoord).x * uColor[0].w;
    oFragColor = vec4(uColor[0].xyz, 1.0) * alpha;
}

