use bevy_asset::Handle;
use bevy_ecs::resource::Resource;
use bevy_render::render_resource::BindGroupLayoutDescriptor;
use bevy_shader::Shader;
use bevy_sprite_render::Mesh2dPipeline;

#[derive(Clone, Resource)]
struct TextGizmoPipeline {
    mesh_pipeline: Mesh2dPipeline,
    uniform_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}
