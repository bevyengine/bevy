#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!












precision highp float;
precision highp sampler2D;

uniform sampler2D uTexture;
uniform vec4 uColor;

in vec2 vTexCoord;

out vec4 oFragColor;

void main(){
    float alpha = texture(uTexture, vTexCoord). r * uColor . a;
    oFragColor = alpha * vec4(uColor . rgb, 1.0);
}

