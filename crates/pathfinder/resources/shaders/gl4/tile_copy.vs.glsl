#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uTileSize[1];
uniform vec4 uTransform[4];
layout(location = 0) in ivec2 aTilePosition;

void main()
{
    vec2 position = vec2(aTilePosition) * uTileSize[0].xy;
    gl_Position = mat4(uTransform[0], uTransform[1], uTransform[2], uTransform[3]) * vec4(position, 0.0, 1.0);
}

