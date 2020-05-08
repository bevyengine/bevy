#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!












precision highp float;
precision highp sampler2D;

uniform vec4 uGroundColor;
uniform vec4 uGridlineColor;

in vec2 vTexCoord;

out vec4 oFragColor;

void main(){
    vec2 texCoordPx = fract(vTexCoord)/ fwidth(vTexCoord);
    oFragColor = any(lessThanEqual(texCoordPx, vec2(1.0)))? uGridlineColor : uGroundColor;
}

