#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;

uniform mat4 uTransform;
uniform vec2 uTileSize;

in ivec2 aTilePosition;

void main(){
    vec2 position = vec2(aTilePosition)* uTileSize;
    gl_Position = uTransform * vec4(position, 0.0, 1.0);
}

