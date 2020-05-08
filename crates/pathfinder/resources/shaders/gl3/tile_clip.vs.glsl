#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!
#version 450











precision highp float;
precision highp sampler2D;

in ivec2 aTileOffset;
in ivec2 aDestTileOrigin;
in ivec2 aSrcTileOrigin;
in int aSrcBackdrop;

out vec2 vTexCoord;
out float vBackdrop;

void main(){
    vec2 destPosition = vec2(aDestTileOrigin + aTileOffset)/ vec2(256.0);
    vec2 srcPosition = vec2(aSrcTileOrigin + aTileOffset)/ vec2(256.0);
    vTexCoord = srcPosition;
    vBackdrop = float(aSrcBackdrop);
    gl_Position = vec4(mix(vec2(- 1.0), vec2(1.0), destPosition), 0.0, 1.0);
}

