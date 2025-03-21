mod command;
mod instancing;
mod pipeline;
mod prepare_material;
mod specialization;

use core::{hash::Hash, marker::PhantomData};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{prelude::AssetChanged, AssetEvents};
use bevy_core_pipeline::core_2d::{
    AlphaMask2d, AlphaMask2dBinKey, BatchSetKey2d, Opaque2d, Opaque2dBinKey, Transparent2d,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, Or},
    schedule::IntoScheduleConfigs,
    system::{Query, Res, ResMut, SystemChangeTick},
};
use bevy_math::FloatOrd;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    mesh::{Mesh2d, RenderMesh},
    render_asset::{prepare_assets, RenderAssetPlugin, RenderAssets},
    render_phase::{
        AddRenderCommand, BinnedRenderPhaseType, DrawFunctionId, InputUniformIndex,
        PhaseItemExtraIndex, ViewBinnedRenderPhases, ViewSortedRenderPhases,
    },
    render_resource::{BindGroupId, PipelineCache, SpecializedMeshPipelines},
    sync_world::MainEntity,
    view::{ExtractedView, RenderVisibleEntities},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use instancing::{extract_mesh_materials_2d, RenderMaterial2dInstances};
use pipeline::{cache::SpecializedMaterial2dPipelineCache, DrawMaterial2d, Material2dPipeline};
use prepare_material::PreparedMaterial2d;

use crate::mesh::{
    Mesh2dPipelineKey, RenderMesh2dInstances, ViewKeyCache, ViewSpecializationTicks,
};

use super::{Material2d, MeshMaterial2d};

pub use pipeline::Material2dKey;
use specialization::{EntitiesNeedingSpecialization, EntitySpecializationTicks};

/// Sets up the rendering logic for a [`Material2d`]
pub struct Material2dRenderingPlugin<M: Material2d>(PhantomData<M>);

impl<M: Material2d> Default for Material2dRenderingPlugin<M> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<M: Material2d> Plugin for Material2dRenderingPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<EntitiesNeedingSpecialization<M>>()
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
                        extract_entities_needs_specialization::<M>,
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

pub fn extract_entities_needs_specialization<M>(
    entities_needing_specialization: Extract<Res<EntitiesNeedingSpecialization<M>>>,
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks<M>>,
    ticks: SystemChangeTick,
) where
    M: Material2d,
{
    for entity in entities_needing_specialization.iter() {
        // Update the entity's specialization tick with this run's tick
        entity_specialization_ticks.insert((*entity).into(), ticks.this_run());
    }
}

pub fn check_entities_needing_specialization<M>(
    needs_specialization: Query<
        Entity,
        Or<(
            Changed<Mesh2d>,
            AssetChanged<Mesh2d>,
            Changed<MeshMaterial2d<M>>,
            AssetChanged<MeshMaterial2d<M>>,
        )>,
    >,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<M>>,
) where
    M: Material2d,
{
    entities_needing_specialization.clear();
    for entity in &needs_specialization {
        entities_needing_specialization.push(entity);
    }
}

pub fn specialize_material2d_meshes<M: Material2d>(
    material2d_pipeline: Res<Material2dPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<Material2dPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    (render_meshes, render_materials): (
        Res<RenderAssets<RenderMesh>>,
        Res<RenderAssets<PreparedMaterial2d<M>>>,
    ),
    mut render_mesh_instances: ResMut<RenderMesh2dInstances>,
    render_material_instances: Res<RenderMaterial2dInstances<M>>,
    transparent_render_phases: Res<ViewSortedRenderPhases<Transparent2d>>,
    opaque_render_phases: Res<ViewBinnedRenderPhases<Opaque2d>>,
    alpha_mask_render_phases: Res<ViewBinnedRenderPhases<AlphaMask2d>>,
    views: Query<(&MainEntity, &ExtractedView, &RenderVisibleEntities)>,
    view_key_cache: Res<ViewKeyCache>,
    entity_specialization_ticks: Res<EntitySpecializationTicks<M>>,
    view_specialization_ticks: Res<ViewSpecializationTicks>,
    ticks: SystemChangeTick,
    mut specialized_material_pipeline_cache: ResMut<SpecializedMaterial2dPipelineCache<M>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    if render_material_instances.is_empty() {
        return;
    }

    for (view_entity, view, visible_entities) in &views {
        if !transparent_render_phases.contains_key(&view.retained_view_entity)
            && !opaque_render_phases.contains_key(&view.retained_view_entity)
            && !alpha_mask_render_phases.contains_key(&view.retained_view_entity)
        {
            continue;
        }

        let Some(view_key) = view_key_cache.get(view_entity) else {
            continue;
        };

        let view_tick = view_specialization_ticks.get(view_entity).unwrap();
        let view_specialized_material_pipeline_cache = specialized_material_pipeline_cache
            .entry(*view_entity)
            .or_default();

        for (_, visible_entity) in visible_entities.iter::<Mesh2d>() {
            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let entity_tick = entity_specialization_ticks.get(visible_entity).unwrap();
            let last_specialized_tick = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .map(|(tick, _)| *tick);
            let needs_specialization = last_specialized_tick.is_none_or(|tick| {
                view_tick.is_newer_than(tick, ticks.this_run())
                    || entity_tick.is_newer_than(tick, ticks.this_run())
            });
            if !needs_specialization {
                continue;
            }
            let Some(mesh_instance) = render_mesh_instances.get_mut(visible_entity) else {
                continue;
            };
            let Some(material_2d) = render_materials.get(*material_asset_id) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let mesh_key = *view_key
                | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology())
                | material_2d.properties.mesh_pipeline_key_bits;

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &material2d_pipeline,
                Material2dKey {
                    mesh_key,
                    bind_group_data: material_2d.key.clone(),
                },
                &mesh.layout,
            );

            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    tracing::error!("{}", err);
                    continue;
                }
            };

            view_specialized_material_pipeline_cache
                .insert(*visible_entity, (ticks.this_run(), pipeline_id));
        }
    }
}

pub fn queue_material2d_meshes<M: Material2d>(
    (render_meshes, render_materials): (
        Res<RenderAssets<RenderMesh>>,
        Res<RenderAssets<PreparedMaterial2d<M>>>,
    ),
    mut render_mesh_instances: ResMut<RenderMesh2dInstances>,
    render_material_instances: Res<RenderMaterial2dInstances<M>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque2d>>,
    mut alpha_mask_render_phases: ResMut<ViewBinnedRenderPhases<AlphaMask2d>>,
    views: Query<(&MainEntity, &ExtractedView, &RenderVisibleEntities)>,
    specialized_material_pipeline_cache: ResMut<SpecializedMaterial2dPipelineCache<M>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    if render_material_instances.is_empty() {
        return;
    }

    for (view_entity, view, visible_entities) in &views {
        let Some(view_specialized_material_pipeline_cache) =
            specialized_material_pipeline_cache.get(view_entity)
        else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(alpha_mask_phase) = alpha_mask_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        for (render_entity, visible_entity) in visible_entities.iter::<Mesh2d>() {
            let Some((current_change_tick, pipeline_id)) = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .map(|(current_change_tick, pipeline_id)| (*current_change_tick, *pipeline_id))
            else {
                continue;
            };

            // Skip the entity if it's cached in a bin and up to date.
            if opaque_phase.validate_cached_entity(*visible_entity, current_change_tick)
                || alpha_mask_phase.validate_cached_entity(*visible_entity, current_change_tick)
            {
                continue;
            }

            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.get_mut(visible_entity) else {
                continue;
            };
            let Some(material_2d) = render_materials.get(*material_asset_id) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            mesh_instance.material_bind_group_id = material_2d.get_bind_group_id();
            let mesh_z = mesh_instance.transforms.world_from_local.translation.z;

            // We don't support multidraw yet for 2D meshes, so we use this
            // custom logic to generate the `BinnedRenderPhaseType` instead of
            // `BinnedRenderPhaseType::mesh`, which can return
            // `BinnedRenderPhaseType::MultidrawableMesh` if the hardware
            // supports multidraw.
            let binned_render_phase_type = if mesh_instance.automatic_batching {
                BinnedRenderPhaseType::BatchableMesh
            } else {
                BinnedRenderPhaseType::UnbatchableMesh
            };

            match material_2d.properties.alpha_mode {
                AlphaMode2d::Opaque => {
                    let bin_key = Opaque2dBinKey {
                        pipeline: pipeline_id,
                        draw_function: material_2d.properties.draw_function_id,
                        asset_id: mesh_instance.mesh_asset_id.into(),
                        material_bind_group_id: material_2d.get_bind_group_id().0,
                    };
                    opaque_phase.add(
                        BatchSetKey2d {
                            indexed: mesh.indexed(),
                        },
                        bin_key,
                        (*render_entity, *visible_entity),
                        InputUniformIndex::default(),
                        binned_render_phase_type,
                        current_change_tick,
                    );
                }
                AlphaMode2d::Mask(_) => {
                    let bin_key = AlphaMask2dBinKey {
                        pipeline: pipeline_id,
                        draw_function: material_2d.properties.draw_function_id,
                        asset_id: mesh_instance.mesh_asset_id.into(),
                        material_bind_group_id: material_2d.get_bind_group_id().0,
                    };
                    alpha_mask_phase.add(
                        BatchSetKey2d {
                            indexed: mesh.indexed(),
                        },
                        bin_key,
                        (*render_entity, *visible_entity),
                        InputUniformIndex::default(),
                        binned_render_phase_type,
                        current_change_tick,
                    );
                }
                AlphaMode2d::Blend => {
                    transparent_phase.add(Transparent2d {
                        entity: (*render_entity, *visible_entity),
                        draw_function: material_2d.properties.draw_function_id,
                        pipeline: pipeline_id,
                        // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                        // lowest sort key and getting closer should increase. As we have
                        // -z in front of the camera, the largest distance is -far with values increasing toward the
                        // camera. As such we can just use mesh_z as the distance
                        sort_key: FloatOrd(mesh_z + material_2d.properties.depth_bias),
                        // Batching is done in batch_and_prepare_render_phase
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::None,
                        extracted_index: usize::MAX,
                        indexed: mesh.indexed(),
                    });
                }
            }
        }
    }
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct Material2dBindGroupId(pub Option<BindGroupId>);

/// Common [`Material2d`] properties, calculated for a specific material instance.
pub struct Material2dProperties {
    /// The [`AlphaMode2d`] of this material.
    pub alpha_mode: AlphaMode2d,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may
    pub depth_bias: f32,
    /// The bits in the [`Mesh2dPipelineKey`] for this material.
    ///
    /// These are precalculated so that we can just "or" them together in
    /// [`queue_material2d_meshes`].
    pub mesh_pipeline_key_bits: Mesh2dPipelineKey,
    pub draw_function_id: DrawFunctionId,
}

/// Sets how a 2d material's base color alpha channel is used for transparency.
/// Currently, this only works with [`Mesh2d`]. Sprites are always transparent.
///
/// This is very similar to [`AlphaMode`](bevy_render::alpha::AlphaMode) but this only applies to 2d meshes.
/// We use a separate type because 2d doesn't support all the transparency modes that 3d does.
#[derive(Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Default, Debug, Clone)]
pub enum AlphaMode2d {
    /// Base color alpha values are overridden to be fully opaque (1.0).
    #[default]
    Opaque,
    /// Reduce transparency to fully opaque or fully transparent
    /// based on a threshold.
    ///
    /// Compares the base color alpha value to the specified threshold.
    /// If the value is below the threshold,
    /// considers the color to be fully transparent (alpha is set to 0.0).
    /// If it is equal to or above the threshold,
    /// considers the color to be fully opaque (alpha is set to 1.0).
    Mask(f32),
    /// The base color alpha value defines the opacity of the color.
    /// Standard alpha-blending is used to blend the fragment's color
    /// with the color behind it.
    Blend,
}
