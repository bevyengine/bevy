use bevy_ecs::component::Component;
use bevy_render::{render_resource::BindGroupLayout, renderer::RenderDevice};

#[derive(Component)]
pub struct SolariGlobalIlluminationViewResources {}

pub fn prepare_resources() {
    todo!()
}

pub fn create_bind_group_layouts(
    render_device: &RenderDevice,
) -> (BindGroupLayout, BindGroupLayout) {
    todo!()
}

#[derive(Component)]
pub struct SolariGlobalIlluminationBindGroups {}

pub fn prepare_bind_groups() {
    todo!()
}
