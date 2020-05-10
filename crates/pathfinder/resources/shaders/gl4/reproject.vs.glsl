#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uNewTransform[4];
layout(location = 0) in ivec2 aPosition;
layout(location = 0) out vec2 vTexCoord;

void main()
{
    vec2 position = vec2(aPosition);
    vTexCoord = position;
    gl_Position = mat4(uNewTransform[0], uNewTransform[1], uNewTransform[2], uNewTransform[3]) * vec4(position, 0.0, 1.0);
}

