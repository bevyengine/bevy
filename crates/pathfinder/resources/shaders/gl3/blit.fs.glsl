#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;

uniform sampler2D uSrc;


in vec2 vTexCoord;

out vec4 oFragColor;

void main(){



    vec4 color = texture(uSrc, vTexCoord);


    oFragColor = vec4(color . rgb * color . a, color . a);
}

