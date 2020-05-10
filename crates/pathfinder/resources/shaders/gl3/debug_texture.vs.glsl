#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uTextureSize[1];
uniform vec4 uFramebufferSize[1];
out vec2 vTexCoord;
layout(location = 1) in ivec2 aTexCoord;
layout(location = 0) in ivec2 aPosition;

void main()
{
    vTexCoord = vec2(aTexCoord) / uTextureSize[0].xy;
    vec2 position = ((vec2(aPosition) / uFramebufferSize[0].xy) * 2.0) - vec2(1.0);
    gl_Position = vec4(position.x, -position.y, 0.0, 1.0);
}

