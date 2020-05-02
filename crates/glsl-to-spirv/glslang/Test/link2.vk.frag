#version 450

uniform sampler2D s2D;

vec4 getColor()
{
  return texture(s2D, vec2(0.5));
}
