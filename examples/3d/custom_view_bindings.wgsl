struct CustomViewUniform {
    time: f32,
};

@group(0) @binding(0)
var<uniform> custom_view_binding: CustomViewUniform;