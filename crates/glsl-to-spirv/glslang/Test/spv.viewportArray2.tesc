#version 450
#extension GL_NV_viewport_array2 :require

layout(vertices = 4) out;

out gl_PerVertex {
    int gl_ViewportMask[2];
    int gl_ViewportIndex;
    layout (viewport_relative) out highp int gl_Layer;
} gl_out[4];

void main()
{
    gl_out[gl_InvocationID].gl_ViewportMask[0]                  = 1;
    gl_out[gl_InvocationID].gl_ViewportIndex                    = 2;
}
