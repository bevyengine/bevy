#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 3) in vec4 Vertex_Tangent;
#endif

layout(location = 0) out vec3 v_WorldPosition;
layout(location = 1) out vec3 v_WorldNormal;
layout(location = 2) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 3) out vec4 v_WorldTangent;
#endif

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

// inverts the scaling, but keeps the rotation of a mat3
// ! assumes that m is an orthogonal matrix
mat3 to_normal_matrix(mat3 m) {
    vec3 fac = 1.0 / vec3(
    	dot(m[0], m[0]),
    	dot(m[1], m[1]),
    	dot(m[2], m[2])
    );
    m[0] *= fac.x;
    m[1] *= fac.y;
    m[2] *= fac.z;
    return m;
}

void main() {
    vec4 world_position = Model * vec4(Vertex_Position, 1.0);
    v_WorldPosition = world_position.xyz;
    v_WorldNormal = to_normal_matrix(mat3(Model)) * Vertex_Normal;
    v_Uv = Vertex_Uv;
#ifdef STANDARDMATERIAL_NORMAL_MAP
    v_WorldTangent = vec4(mat3(Model) * Vertex_Tangent.xyz, Vertex_Tangent.w);
#endif
    gl_Position = ViewProj * world_position;
}
