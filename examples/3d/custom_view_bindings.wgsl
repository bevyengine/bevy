#define_import_path bevy_pbr::mesh_view_user_bindings

struct CustomViewUniform {
    time: f32,
};

@group(0) @binding(9)
var<uniform> custom_view_binding: CustomViewUniform;