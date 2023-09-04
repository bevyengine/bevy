use self::{
    bind_group::{queue_scene_bind_group, SolariSceneBindGroup},
    bind_group_layout::SolariSceneBindGroupLayout,
    blas::{prepare_blas, BlasStorage},
};
use super::SolariEnabled;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::{prelude::resource_exists, schedule::IntoSystemConfigs};
use bevy_reflect::TypeUuid;
use bevy_render::{render_resource::Shader, Render, RenderApp, RenderSet};

mod bind_group;
mod bind_group_layout;
mod blas;
mod helpers;
mod scene_types;

pub struct SolariScenePlugin;

const SOLARI_SCENE_BINDINGS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1717171717171755);

impl Plugin for SolariScenePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SOLARI_SCENE_BINDINGS_SHADER,
            "scene_bindings.wgsl",
            Shader::from_wgsl
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<BlasStorage>()
            .init_resource::<SolariSceneBindGroupLayout>()
            .init_resource::<SolariSceneBindGroup>()
            .add_systems(
                Render,
                (
                    prepare_blas.in_set(RenderSet::PrepareResources),
                    queue_scene_bind_group.in_set(RenderSet::PrepareBindGroups),
                )
                    .run_if(resource_exists::<SolariEnabled>()),
            );
    }
}
