use bevy_core_pipeline::{
    core_2d::Camera2d,
    tonemapping::{get_lut_bindings, Tonemapping, TonemappingLuts},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_render::{
    batching::no_gpu_preprocessing::BatchedInstanceBuffer,
    globals::GlobalsBuffer,
    render_asset::RenderAssets,
    render_resource::{BindGroup, BindGroupEntries, BindGroupId},
    renderer::RenderDevice,
    texture::{FallbackImage, GpuImage},
    view::{ExtractedView, ViewUniforms},
};

use super::{pipeline::Mesh2dPipeline, shader_type::Mesh2dUniform};

#[derive(Resource)]
pub struct Mesh2dBindGroup {
    pub value: BindGroup,
}

#[derive(Component)]
pub struct Mesh2dViewBindGroup {
    pub value: BindGroup,
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct Material2dBindGroupId(pub Option<BindGroupId>);

pub(super) fn prepare_mesh2d_bind_group(
    mut commands: Commands,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    render_device: Res<RenderDevice>,
    mesh2d_uniforms: Res<BatchedInstanceBuffer<Mesh2dUniform>>,
) {
    if let Some(binding) = mesh2d_uniforms.instance_data_binding() {
        commands.insert_resource(Mesh2dBindGroup {
            value: render_device.create_bind_group(
                "mesh2d_bind_group",
                &mesh2d_pipeline.mesh_layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

pub(super) fn prepare_mesh2d_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &Tonemapping), (With<ExtractedView>, With<Camera2d>)>,
    globals_buffer: Res<GlobalsBuffer>,
    tonemapping_luts: Res<TonemappingLuts>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
) {
    let (Some(view_binding), Some(globals)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) else {
        return;
    };

    for (entity, tonemapping) in &views {
        let lut_bindings =
            get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
        let view_bind_group = render_device.create_bind_group(
            "mesh2d_view_bind_group",
            &mesh2d_pipeline.view_layout,
            &BindGroupEntries::with_indices((
                (0, view_binding.clone()),
                (1, globals.clone()),
                (2, lut_bindings.0),
                (3, lut_bindings.1),
            )),
        );

        commands.entity(entity).insert(Mesh2dViewBindGroup {
            value: view_bind_group,
        });
    }
}
