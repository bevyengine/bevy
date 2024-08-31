#define_import_path bevy_pbr::atmosphere::bindings

@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
@group(0) @binding(1) var<uniform> view_uniforms: ViewUniforms; //TODO: import view uniform type
@group(0) @binding(2) var<storage>
