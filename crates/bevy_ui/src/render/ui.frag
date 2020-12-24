#version 450

layout(location = 0) in vec2 v_Uv;

struct BorderData {
    vec4 size;
    vec4 left_color;
    vec4 right_color;
    vec4 top_color;
    vec4 bottom_color;
};

layout(set = 1, binding = 1) uniform Node_size {
    vec2 NodeSize;
};
layout(set = 1, binding = 2) uniform RenderBorders {
    BorderData border;
};

layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform ColorMaterial_color {
    vec4 Color;
};

# ifdef COLORMATERIAL_TEXTURE 
layout(set = 2, binding = 1) uniform texture2D ColorMaterial_texture;
layout(set = 2, binding = 2) uniform sampler ColorMaterial_texture_sampler;
# endif

void main() {
    vec4 color = Color;
# ifdef COLORMATERIAL_TEXTURE
    color *= texture(
        sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),
        v_Uv);
# endif
    float r = NodeSize.x / NodeSize.y;
    vec2 localUvCoords = abs((v_Uv - 0.5.xx) * 2);

    float closerY = step(0.0, r * localUvCoords.x + 1.0 - r - localUvCoords.y);
    float closerX = 1 - closerY;

    vec2 localCoords = NodeSize * v_Uv;
    vec2 centerCoords = localCoords - NodeSize / 2;

    float closerRight = step(0.0, centerCoords.x);
    float closerLeft = 1 - closerRight;
    float closerBottom = step(0.0, centerCoords.y);
    float closerTop = 1 - closerBottom;

    vec4 borderColor =  closerX * (closerLeft * border.left_color + closerRight * border.right_color) + 
                        closerY * (closerTop * border.top_color + closerBottom * border.bottom_color);
    
    vec4 borderSize = border.size;
    float _inBorder = closerX * (closerTop * (borderSize[0] - localCoords.y) + closerBottom * (borderSize[1] - (NodeSize - localCoords).y))
                    + closerY * (closerLeft * (borderSize[2] - localCoords.x) + closerRight * (borderSize[3] - (NodeSize - localCoords).x));
    float inBorder = 1.0 - step(0.0, -_inBorder);
    float outOfBorder = 1 - inBorder;

    o_Target = color * outOfBorder + borderColor * inBorder;
    //o_Target = color;
}
