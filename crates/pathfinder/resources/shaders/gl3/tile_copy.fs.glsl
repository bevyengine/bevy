#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uFramebufferSize[1];
uniform sampler2D SPIRV_Cross_CombineduSrcuSampler;

layout(location = 0) out vec4 oFragColor;

void main()
{
    vec2 texCoord = gl_FragCoord.xy / uFramebufferSize[0].xy;
    oFragColor = texture(SPIRV_Cross_CombineduSrcuSampler, texCoord);
}

