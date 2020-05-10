#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


layout(location = 1) in ivec2 aDestTileOrigin;
layout(location = 0) in ivec2 aTileOffset;
layout(location = 2) in ivec2 aSrcTileOrigin;
layout(location = 0) out vec2 vTexCoord;
layout(location = 1) out float vBackdrop;
layout(location = 3) in int aSrcBackdrop;

void main()
{
    vec2 destPosition = vec2(aDestTileOrigin + aTileOffset) / vec2(256.0);
    vec2 srcPosition = vec2(aSrcTileOrigin + aTileOffset) / vec2(256.0);
    vTexCoord = srcPosition;
    vBackdrop = float(aSrcBackdrop);
    gl_Position = vec4(mix(vec2(-1.0), vec2(1.0), destPosition), 0.0, 1.0);
}

