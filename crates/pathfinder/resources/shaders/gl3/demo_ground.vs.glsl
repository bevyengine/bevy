#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform ivec4 uGridlineCount[1];
uniform vec4 uTransform[4];
out vec2 vTexCoord;
layout(location = 0) in ivec2 aPosition;

void main()
{
    vTexCoord = vec2(aPosition * ivec2(uGridlineCount[0].x));
    gl_Position = mat4(uTransform[0], uTransform[1], uTransform[2], uTransform[3]) * vec4(ivec4(aPosition.x, 0, aPosition.y, 1));
}

