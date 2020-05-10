#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uColor[1];
layout(location = 0) out vec4 oFragColor;

void main()
{
    oFragColor = vec4(uColor[0].xyz, 1.0) * uColor[0].w;
}

