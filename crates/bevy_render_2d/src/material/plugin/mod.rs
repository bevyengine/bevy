mod systems;

use core::{hash::Hash, marker::PhantomData};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetApp, AssetEvents};
use bevy_core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_render::{
    camera::extract_cameras,
    mesh::RenderMesh,
    render_asset::{prepare_assets, RenderAssetPlugin},
    render_phase::AddRenderCommand,
    render_resource::SpecializedMeshPipelines,
    ExtractSchedule, Render, RenderApp, RenderSet,
};

use super::{
    commands::DrawMaterial2d,
    pipeline::Material2dPipeline,
    render::{
        EntitiesNeedingSpecialization, EntitySpecializationTicks, PreparedMaterial2d,
        RenderMaterial2dInstances, SpecializedMaterial2dPipelineCache,
    },
    Material2d, MeshMaterial2d,
};

use systems::*;

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`Material2d`]
/// asset type (which includes [`Material2d`] types).
pub struct Material2dPlugin<M: Material2d>(PhantomData<M>);

impl<M: Material2d> Default for Material2dPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: Material2d> Plugin for Material2dPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<M>()
            .init_resource::<EntitiesNeedingSpecialization<M>>()
            .register_type::<MeshMaterial2d<M>>()
            .add_plugins(RenderAssetPlugin::<PreparedMaterial2d<M>>::default())
            .add_systems(
                PostUpdate,
                check_entities_needing_specialization::<M>.after(AssetEvents),
            );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<EntitySpecializationTicks<M>>()
                .init_resource::<SpecializedMaterial2dPipelineCache<M>>()
                .add_render_command::<Opaque2d, DrawMaterial2d<M>>()
                .add_render_command::<AlphaMask2d, DrawMaterial2d<M>>()
                .add_render_command::<Transparent2d, DrawMaterial2d<M>>()
                .init_resource::<RenderMaterial2dInstances<M>>()
                .init_resource::<SpecializedMeshPipelines<Material2dPipeline<M>>>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_entities_needs_specialization::<M>.after(extract_cameras),
                        extract_mesh_materials_2d::<M>,
                    ),
                )
                .add_systems(
                    Render,
                    (
                        specialize_material2d_meshes::<M>
                            .in_set(RenderSet::PrepareMeshes)
                            .after(prepare_assets::<PreparedMaterial2d<M>>)
                            .after(prepare_assets::<RenderMesh>),
                        queue_material2d_meshes::<M>
                            .in_set(RenderSet::QueueMeshes)
                            .after(prepare_assets::<PreparedMaterial2d<M>>),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<Material2dPipeline<M>>();
        }
    }
}
