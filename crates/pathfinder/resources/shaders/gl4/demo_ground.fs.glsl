#version {{version}}
// Automatically generated from files in pathfinder/shaders/. Do not edit!


uniform vec4 uGridlineColor[1];
uniform vec4 uGroundColor[1];
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 oFragColor;

void main()
{
    vec2 texCoordPx = fract(vTexCoord) / fwidth(vTexCoord);
    vec4 _28;
    if (any(lessThanEqual(texCoordPx, vec2(1.0))))
    {
        _28 = uGridlineColor[0];
    }
    else
    {
        _28 = uGroundColor[0];
    }
    oFragColor = _28;
}

