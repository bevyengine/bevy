#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};
layout(set = 2, binding = 1) uniform Sprite {
    vec2 size;
    uint flip;
};

void main() {
    vec2 uv = Vertex_Uv;

    // Flip the sprite if necessary by flipping the UVs

    uint x_flip_bit = 1; // The X flip bit
    uint y_flip_bit = 2; // The Y flip bit

    // Note: Here we subtract f32::EPSILON from the flipped UV coord. This is due to reasons unknown
    // to me (@zicklag ) that causes the uv's to be slightly offset and causes over/under running of
    // the sprite UV sampling which is visible when resizing the screen.
    float epsilon = 0.00000011920929;
    if ((flip & x_flip_bit) == x_flip_bit) {
        uv = vec2(1.0 - uv.x - epsilon, uv.y);
    }
    if ((flip & y_flip_bit) == y_flip_bit) {
        uv = vec2(uv.x, 1.0 - uv.y - epsilon);
    }

    v_Uv = uv;

    vec3 position = Vertex_Position * vec3(size, 1.0);
    gl_Position = ViewProj * Model * vec4(position, 1.0);
}
