#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;

in ivec2 aPosition;

out vec2 vTexCoord;

void main(){
    vec2 texCoord = vec2(aPosition);



    vTexCoord = texCoord;
    gl_Position = vec4(mix(vec2(- 1.0), vec2(1.0), vec2(aPosition)), 0.0, 1.0);
}

