#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!












precision highp float;
precision highp sampler2D;

uniform vec4 uColor;

out vec4 oFragColor;

void main(){
    oFragColor = vec4(uColor . rgb, 1.0)* uColor . a;
}

