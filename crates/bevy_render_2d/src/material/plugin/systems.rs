use core::hash::Hash;

use bevy_asset::prelude::AssetChanged;
use bevy_core_pipeline::core_2d::{
    AlphaMask2d, AlphaMask2dBinKey, BatchSetKey2d, Opaque2d, Opaque2dBinKey, Transparent2d,
};
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or, With},
    removal_detection::RemovedComponents,
    system::{Local, Query, Res, ResMut, SystemChangeTick},
};
use bevy_math::FloatOrd;
use bevy_render::{
    mesh::{Mesh2d, RenderMesh},
    render_asset::RenderAssets,
    render_phase::{
        BinnedRenderPhaseType, InputUniformIndex, PhaseItemExtraIndex, ViewBinnedRenderPhases,
        ViewSortedRenderPhases,
    },
    render_resource::{PipelineCache, SpecializedMeshPipelines},
    sync_world::MainEntity,
    view::{ExtractedView, RenderVisibleEntities, ViewVisibility},
    Extract,
};
use bevy_utils::Parallel;

use crate::{
    material::{
        instances::RenderMaterial2dInstances,
        key::Material2dKey,
        pipeline::Material2dPipeline,
        prepared_asset::PreparedMaterial2d,
        specialization::{
            EntitiesNeedingSpecialization, EntitySpecializationTicks,
            SpecializedMaterial2dPipelineCache,
        },
        AlphaMode2d, Material2d, MeshMaterial2d,
    },
    mesh_pipeline::{
        key::Mesh2dPipelineKey,
        render::{RenderMesh2dInstances, ViewKeyCache, ViewSpecializationTicks},
    },
};

pub fn extract_mesh_materials_2d<M: Material2d>(
    mut material_instances: ResMut<RenderMaterial2dInstances<M>>,
    changed_meshes_query: Extract<
        Query<
            (Entity, &ViewVisibility, &MeshMaterial2d<M>),
            Or<(Changed<ViewVisibility>, Changed<MeshMaterial2d<M>>)>,
        >,
    >,
    mut removed_visibilities_query: Extract<RemovedComponents<ViewVisibility>>,
    mut removed_materials_query: Extract<RemovedComponents<MeshMaterial2d<M>>>,
) {
    for (entity, view_visibility, material) in &changed_meshes_query {
        if view_visibility.get() {
            add_mesh_instance(entity, material, &mut material_instances);
        } else {
            remove_mesh_instance(entity, &mut material_instances);
        }
    }

    for entity in removed_visibilities_query
        .read()
        .chain(removed_materials_query.read())
    {
        // Only queue a mesh for removal if we didn't pick it up above.
        // It's possible that a necessary component was removed and re-added in
        // the same frame.
        if !changed_meshes_query.contains(entity) {
            remove_mesh_instance(entity, &mut material_instances);
        }
    }

    // Adds or updates a mesh instance in the [`RenderMaterial2dInstances`]
    // array.
    fn add_mesh_instance<M>(
        entity: Entity,
        material: &MeshMaterial2d<M>,
        material_instances: &mut RenderMaterial2dInstances<M>,
    ) where
        M: Material2d,
    {
        material_instances.insert(entity.into(), material.id());
    }

    // Removes a mesh instance from the [`RenderMaterial2dInstances`] array.
    fn remove_mesh_instance<M>(
        entity: Entity,
        material_instances: &mut RenderMaterial2dInstances<M>,
    ) where
        M: Material2d,
    {
        material_instances.remove(&MainEntity::from(entity));
    }
}

pub fn extract_entities_needs_specialization<M>(
    entities_needing_specialization: Extract<Res<EntitiesNeedingSpecialization<M>>>,
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks<M>>,
    mut removed_mesh_material_components: Extract<RemovedComponents<MeshMaterial2d<M>>>,
    mut specialized_material2d_pipeline_cache: ResMut<SpecializedMaterial2dPipelineCache<M>>,
    views: Query<&MainEntity, With<ExtractedView>>,
    ticks: SystemChangeTick,
) where
    M: Material2d,
{
    for entity in entities_needing_specialization.iter() {
        // Update the entity's specialization tick with this run's tick
        entity_specialization_ticks.insert((*entity).into(), ticks.this_run());
    }
    // Clean up any despawned entities
    for entity in removed_mesh_material_components.read() {
        entity_specialization_ticks.remove(&MainEntity::from(entity));
        for view in views {
            if let Some(cache) = specialized_material2d_pipeline_cache.get_mut(view) {
                cache.remove(&MainEntity::from(entity));
            }
        }
    }
}

pub fn check_entities_needing_specialization<M>(
    needs_specialization: Query<
        Entity,
        (
            Or<(
                Changed<Mesh2d>,
                AssetChanged<Mesh2d>,
                Changed<MeshMaterial2d<M>>,
                AssetChanged<MeshMaterial2d<M>>,
            )>,
            With<MeshMaterial2d<M>>,
        ),
    >,
    mut par_local: Local<Parallel<Vec<Entity>>>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<M>>,
) where
    M: Material2d,
{
    entities_needing_specialization.clear();

    needs_specialization
        .par_iter()
        .for_each(|entity| par_local.borrow_local_mut().push(entity));

    par_local.drain_into(&mut entities_needing_specialization);
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
