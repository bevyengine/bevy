#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;

in vec3 aPosition;

void main(){
    gl_Position = vec4(aPosition, 1.0);
}

