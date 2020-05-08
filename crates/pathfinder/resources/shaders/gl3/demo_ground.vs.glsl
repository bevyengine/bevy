#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!












precision highp float;
precision highp sampler2D;

uniform mat4 uTransform;
uniform int uGridlineCount;

in ivec2 aPosition;

out vec2 vTexCoord;

void main(){
    vTexCoord = vec2(aPosition * uGridlineCount);
    gl_Position = uTransform * vec4(ivec4(aPosition . x, 0, aPosition . y, 1));
}

