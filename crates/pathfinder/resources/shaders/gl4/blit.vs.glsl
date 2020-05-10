#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


layout(location = 0) in ivec2 aPosition;
layout(location = 0) out vec2 vTexCoord;

void main()
{
    vec2 texCoord = vec2(aPosition);
    vTexCoord = texCoord;
    gl_Position = vec4(mix(vec2(-1.0), vec2(1.0), vec2(aPosition)), 0.0, 1.0);
}

