#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uFramebufferSize[1];
layout(location = 0) in ivec2 aPosition;

void main()
{
    vec2 position = ((vec2(aPosition) / uFramebufferSize[0].xy) * 2.0) - vec2(1.0);
    gl_Position = vec4(position.x, -position.y, 0.0, 1.0);
}

