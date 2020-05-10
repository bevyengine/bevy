#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


layout(location = 0) in vec3 aPosition;

void main()
{
    gl_Position = vec4(aPosition, 1.0);
}

