import super::util::make_polka_dots;
import bevy_asset::embedded::shader_material_wesl::files::vertex_attr;
import package::shaders::custom_material_import::CustomMaterial;

@group(3) @binding(0) var<uniform> material: CustomMaterial;

@fragment
fn fragment(
    mesh: vertex_attr::VertexOutput,
) -> @location(0) vec4<f32> {
    return make_polka_dots(mesh.uv, material.time.x);
}
