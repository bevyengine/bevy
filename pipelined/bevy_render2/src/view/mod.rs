pub mod window;

pub use window::*;

use crate::{
    render_resource::DynamicUniformVec,
    renderer::{RenderDevice, RenderQueue},
    RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Vec3};
use bevy_transform::components::GlobalTransform;
use crevice::std140::AsStd140;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .init_resource::<ViewUniforms>()
            .add_system_to_stage(RenderStage::Prepare, prepare_views);
    }
}

pub struct ExtractedView {
    pub projection: Mat4,
    pub transform: GlobalTransform,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, AsStd140)]
pub struct ViewUniform {
    view_proj: Mat4,
    projection: Mat4,
    world_position: Vec3,
}

#[derive(Default)]
pub struct ViewUniforms {
    pub uniforms: DynamicUniformVec<ViewUniform>,
}

pub struct ViewUniformOffset {
    pub offset: u32,
}

fn prepare_views(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<ViewUniforms>,
    mut extracted_views: Query<(Entity, &ExtractedView)>,
) {
    view_uniforms
        .uniforms
        .reserve_and_clear(extracted_views.iter_mut().len(), &render_device);
    for (entity, camera) in extracted_views.iter() {
        let projection = camera.projection;
        let view_uniforms = ViewUniformOffset {
            offset: view_uniforms.uniforms.push(ViewUniform {
                view_proj: projection * camera.transform.compute_matrix().inverse(),
                projection,
                world_position: camera.transform.translation,
            }),
        };

        commands.entity(entity).insert(view_uniforms);
    }

    view_uniforms.uniforms.write_buffer(&render_queue);
}
