#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

use bevy_app::{App, Plugin, PostUpdate};
use bevy_camera::{
    primitives::{Aabb, CascadesFrusta, CubemapFrusta, Frustum, Sphere},
    visibility::{
        CascadesVisibleEntities, CubemapVisibleEntities, InheritedVisibility, NoFrustumCulling,
        PreviousVisibleEntities, RenderLayers, ViewVisibility, VisibilityRange, VisibilitySystems,
        VisibleEntityRanges, VisibleMeshEntities,
    },
    CameraUpdateSystems,
};
use bevy_ecs::{entity::EntityHashSet, prelude::*};
use bevy_math::Vec3A;
use bevy_mesh::Mesh3d;
use bevy_reflect::prelude::*;
use bevy_transform::{components::GlobalTransform, TransformSystems};
use bevy_utils::Parallel;
use core::ops::DerefMut;

pub mod cluster;
pub use cluster::ClusteredDecal;
use cluster::{
    add_clusters, assign::assign_objects_to_clusters, ClusterConfig,
    GlobalVisibleClusterableObjects, VisibleClusterableObjects,
};
mod ambient_light;
pub use ambient_light::AmbientLight;
mod probe;
pub use probe::{EnvironmentMapLight, IrradianceVolume, LightProbe};
mod volumetric;
pub use volumetric::{FogVolume, VolumetricFog, VolumetricLight};
pub mod cascade;
use cascade::{build_directional_light_cascades, clear_directional_light_cascades};
pub use cascade::{CascadeShadowConfig, CascadeShadowConfigBuilder, Cascades};
mod point_light;
pub use point_light::{
    update_point_light_frusta, PointLight, PointLightShadowMap, PointLightTexture,
};
mod spot_light;
pub use spot_light::{
    spot_light_clip_from_view, spot_light_world_from_view, update_spot_light_frusta, SpotLight,
    SpotLightTexture,
};
mod directional_light;
pub use directional_light::{
    update_directional_light_frusta, DirectionalLight, DirectionalLightShadowMap,
    DirectionalLightTexture,
};

/// Constants for operating with the light units: lumens, and lux.
pub mod light_consts {
    /// Approximations for converting the wattage of lamps to lumens.
    ///
    /// The **lumen** (symbol: **lm**) is the unit of [luminous flux], a measure
    /// of the total quantity of [visible light] emitted by a source per unit of
    /// time, in the [International System of Units] (SI).
    ///
    /// For more information, see [wikipedia](https://en.wikipedia.org/wiki/Lumen_(unit))
    ///
    /// [luminous flux]: https://en.wikipedia.org/wiki/Luminous_flux
    /// [visible light]: https://en.wikipedia.org/wiki/Visible_light
    /// [International System of Units]: https://en.wikipedia.org/wiki/International_System_of_Units
    pub mod lumens {
        pub const LUMENS_PER_LED_WATTS: f32 = 90.0;
        pub const LUMENS_PER_INCANDESCENT_WATTS: f32 = 13.8;
        pub const LUMENS_PER_HALOGEN_WATTS: f32 = 19.8;
    }

    /// Predefined for lux values in several locations.
    ///
    /// The **lux** (symbol: **lx**) is the unit of [illuminance], or [luminous flux] per unit area,
    /// in the [International System of Units] (SI). It is equal to one lumen per square meter.
    ///
    /// For more information, see [wikipedia](https://en.wikipedia.org/wiki/Lux)
    ///
    /// [illuminance]: https://en.wikipedia.org/wiki/Illuminance
    /// [luminous flux]: https://en.wikipedia.org/wiki/Luminous_flux
    /// [International System of Units]: https://en.wikipedia.org/wiki/International_System_of_Units
    pub mod lux {
        /// The amount of light (lux) in a moonless, overcast night sky. (starlight)
        pub const MOONLESS_NIGHT: f32 = 0.0001;
        /// The amount of light (lux) during a full moon on a clear night.
        pub const FULL_MOON_NIGHT: f32 = 0.05;
        /// The amount of light (lux) during the dark limit of civil twilight under a clear sky.
        pub const CIVIL_TWILIGHT: f32 = 3.4;
        /// The amount of light (lux) in family living room lights.
        pub const LIVING_ROOM: f32 = 50.;
        /// The amount of light (lux) in an office building's hallway/toilet lighting.
        pub const HALLWAY: f32 = 80.;
        /// The amount of light (lux) in very dark overcast day
        pub const DARK_OVERCAST_DAY: f32 = 100.;
        /// The amount of light (lux) in an office.
        pub const OFFICE: f32 = 320.;
        /// The amount of light (lux) during sunrise or sunset on a clear day.
        pub const CLEAR_SUNRISE: f32 = 400.;
        /// The amount of light (lux) on an overcast day; typical TV studio lighting
        pub const OVERCAST_DAY: f32 = 1000.;
        /// The amount of light (lux) from ambient daylight (not direct sunlight).
        pub const AMBIENT_DAYLIGHT: f32 = 10_000.;
        /// The amount of light (lux) in full daylight (not direct sun).
        pub const FULL_DAYLIGHT: f32 = 20_000.;
        /// The amount of light (lux) in direct sunlight.
        pub const DIRECT_SUNLIGHT: f32 = 100_000.;
        /// The amount of light (lux) of raw sunlight, not filtered by the atmosphere.
        pub const RAW_SUNLIGHT: f32 = 130_000.;
    }
}

pub struct LightPlugin;

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AmbientLight>()
            .register_type::<CascadeShadowConfig>()
            .register_type::<Cascades>()
            .register_type::<DirectionalLight>()
            .register_type::<DirectionalLightShadowMap>()
            .register_type::<NotShadowCaster>()
            .register_type::<NotShadowReceiver>()
            .register_type::<PointLight>()
            .register_type::<LightProbe>()
            .register_type::<EnvironmentMapLight>()
            .register_type::<IrradianceVolume>()
            .register_type::<VolumetricFog>()
            .register_type::<VolumetricLight>()
            .register_type::<PointLightShadowMap>()
            .register_type::<SpotLight>()
            .register_type::<ShadowFilteringMethod>()
            .register_type::<ClusterConfig>()
            .init_resource::<GlobalVisibleClusterableObjects>()
            .init_resource::<AmbientLight>()
            .init_resource::<DirectionalLightShadowMap>()
            .init_resource::<PointLightShadowMap>()
            .configure_sets(
                PostUpdate,
                SimulationLightSystems::UpdateDirectionalLightCascades
                    .ambiguous_with(SimulationLightSystems::UpdateDirectionalLightCascades),
            )
            .configure_sets(
                PostUpdate,
                SimulationLightSystems::CheckLightVisibility
                    .ambiguous_with(SimulationLightSystems::CheckLightVisibility),
            )
            .add_systems(
                PostUpdate,
                (
                    add_clusters
                        .in_set(SimulationLightSystems::AddClusters)
                        .after(CameraUpdateSystems),
                    assign_objects_to_clusters
                        .in_set(SimulationLightSystems::AssignLightsToClusters)
                        .after(TransformSystems::Propagate)
                        .after(VisibilitySystems::CheckVisibility)
                        .after(CameraUpdateSystems),
                    clear_directional_light_cascades
                        .in_set(SimulationLightSystems::UpdateDirectionalLightCascades)
                        .after(TransformSystems::Propagate)
                        .after(CameraUpdateSystems),
                    update_directional_light_frusta
                        .in_set(SimulationLightSystems::UpdateLightFrusta)
                        // This must run after CheckVisibility because it relies on `ViewVisibility`
                        .after(VisibilitySystems::CheckVisibility)
                        .after(TransformSystems::Propagate)
                        .after(SimulationLightSystems::UpdateDirectionalLightCascades)
                        // We assume that no entity will be both a directional light and a spot light,
                        // so these systems will run independently of one another.
                        // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                        .ambiguous_with(update_spot_light_frusta),
                    update_point_light_frusta
                        .in_set(SimulationLightSystems::UpdateLightFrusta)
                        .after(TransformSystems::Propagate)
                        .after(SimulationLightSystems::AssignLightsToClusters),
                    update_spot_light_frusta
                        .in_set(SimulationLightSystems::UpdateLightFrusta)
                        .after(TransformSystems::Propagate)
                        .after(SimulationLightSystems::AssignLightsToClusters),
                    (
                        check_dir_light_mesh_visibility,
                        check_point_light_mesh_visibility,
                    )
                        .in_set(SimulationLightSystems::CheckLightVisibility)
                        .after(VisibilitySystems::CalculateBounds)
                        .after(TransformSystems::Propagate)
                        .after(SimulationLightSystems::UpdateLightFrusta)
                        // NOTE: This MUST be scheduled AFTER the core renderer visibility check
                        // because that resets entity `ViewVisibility` for the first view
                        // which would override any results from this otherwise
                        .after(VisibilitySystems::CheckVisibility)
                        .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
                    build_directional_light_cascades
                        .in_set(SimulationLightSystems::UpdateDirectionalLightCascades)
                        .after(clear_directional_light_cascades),
                ),
            );
    }
}

/// A convenient alias for `Or<(With<PointLight>, With<SpotLight>,
/// With<DirectionalLight>)>`, for use with [`bevy_camera::visibility::VisibleEntities`].
pub type WithLight = Or<(With<PointLight>, With<SpotLight>, With<DirectionalLight>)>;

/// Add this component to make a [`Mesh3d`] not cast shadows.
#[derive(Debug, Component, Reflect, Default)]
#[reflect(Component, Default, Debug)]
pub struct NotShadowCaster;
/// Add this component to make a [`Mesh3d`] not receive shadows.
///
/// **Note:** If you're using diffuse transmission, setting [`NotShadowReceiver`] will
/// cause both “regular” shadows as well as diffusely transmitted shadows to be disabled,
/// even when [`TransmittedShadowReceiver`] is being used.
#[derive(Debug, Component, Reflect, Default)]
#[reflect(Component, Default, Debug)]
pub struct NotShadowReceiver;
/// Add this component to make a [`Mesh3d`] using a PBR material with `StandardMaterial::diffuse_transmission > 0.0`
/// receive shadows on its diffuse transmission lobe. (i.e. its “backside”)
///
/// Not enabled by default, as it requires carefully setting up `StandardMaterial::thickness`
/// (and potentially even baking a thickness texture!) to match the geometry of the mesh, in order to avoid self-shadow artifacts.
///
/// **Note:** Using [`NotShadowReceiver`] overrides this component.
#[derive(Debug, Component, Reflect, Default)]
#[reflect(Component, Default, Debug)]
pub struct TransmittedShadowReceiver;

/// Add this component to a [`Camera3d`](bevy_camera::Camera3d)
/// to control how to anti-alias shadow edges.
///
/// The different modes use different approaches to
/// [Percentage Closer Filtering](https://developer.nvidia.com/gpugems/gpugems/part-ii-lighting-and-shadows/chapter-11-shadow-map-antialiasing).
#[derive(Debug, Component, Reflect, Clone, Copy, PartialEq, Eq, Default)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub enum ShadowFilteringMethod {
    /// Hardware 2x2.
    ///
    /// Fast but poor quality.
    Hardware2x2,
    /// Approximates a fixed Gaussian blur, good when TAA isn't in use.
    ///
    /// Good quality, good performance.
    ///
    /// For directional and spot lights, this uses a [method by Ignacio Castaño
    /// for *The Witness*] using 9 samples and smart filtering to achieve the same
    /// as a regular 5x5 filter kernel.
    ///
    /// [method by Ignacio Castaño for *The Witness*]: https://web.archive.org/web/20230210095515/http://the-witness.net/news/2013/09/shadow-mapping-summary-part-1/
    #[default]
    Gaussian,
    /// A randomized filter that varies over time, good when TAA is in use.
    ///
    /// Good quality when used with `TemporalAntiAliasing`
    /// and good performance.
    ///
    /// For directional and spot lights, this uses a [method by Jorge Jimenez for
    /// *Call of Duty: Advanced Warfare*] using 8 samples in spiral pattern,
    /// randomly-rotated by interleaved gradient noise with spatial variation.
    ///
    /// [method by Jorge Jimenez for *Call of Duty: Advanced Warfare*]: https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/
    Temporal,
}

/// System sets used to run light-related systems.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SimulationLightSystems {
    AddClusters,
    AssignLightsToClusters,
    /// System order ambiguities between systems in this set are ignored:
    /// each [`build_directional_light_cascades`] system is independent of the others,
    /// and should operate on distinct sets of entities.
    UpdateDirectionalLightCascades,
    UpdateLightFrusta,
    /// System order ambiguities between systems in this set are ignored:
    /// the order of systems within this set is irrelevant, as the various visibility-checking systems
    /// assumes that their operations are irreversible during the frame.
    CheckLightVisibility,
}

fn shrink_entities(visible_entities: &mut Vec<Entity>) {
    // Check that visible entities capacity() is no more than two times greater than len()
    let capacity = visible_entities.capacity();
    let reserved = capacity
        .checked_div(visible_entities.len())
        .map_or(0, |reserve| {
            if reserve > 2 {
                capacity / (reserve / 2)
            } else {
                capacity
            }
        });

    visible_entities.shrink_to(reserved);
}

pub fn check_dir_light_mesh_visibility(
    mut commands: Commands,
    mut directional_lights: Query<
        (
            &DirectionalLight,
            &CascadesFrusta,
            &mut CascadesVisibleEntities,
            Option<&RenderLayers>,
            &ViewVisibility,
        ),
        Without<SpotLight>,
    >,
    visible_entity_query: Query<
        (
            Entity,
            &InheritedVisibility,
            Option<&RenderLayers>,
            Option<&Aabb>,
            Option<&GlobalTransform>,
            Has<VisibilityRange>,
            Has<NoFrustumCulling>,
        ),
        (
            Without<NotShadowCaster>,
            Without<DirectionalLight>,
            With<Mesh3d>,
        ),
    >,
    visible_entity_ranges: Option<Res<VisibleEntityRanges>>,
    mut defer_visible_entities_queue: Local<Parallel<Vec<Entity>>>,
    mut view_visible_entities_queue: Local<Parallel<Vec<Vec<Entity>>>>,
) {
    let visible_entity_ranges = visible_entity_ranges.as_deref();

    for (directional_light, frusta, mut visible_entities, maybe_view_mask, light_view_visibility) in
        &mut directional_lights
    {
        let mut views_to_remove = Vec::new();
        for (view, cascade_view_entities) in &mut visible_entities.entities {
            match frusta.frusta.get(view) {
                Some(view_frusta) => {
                    cascade_view_entities.resize(view_frusta.len(), Default::default());
                    cascade_view_entities.iter_mut().for_each(|x| x.clear());
                }
                None => views_to_remove.push(*view),
            };
        }
        for (view, frusta) in &frusta.frusta {
            visible_entities
                .entities
                .entry(*view)
                .or_insert_with(|| vec![VisibleMeshEntities::default(); frusta.len()]);
        }

        for v in views_to_remove {
            visible_entities.entities.remove(&v);
        }

        // NOTE: If shadow mapping is disabled for the light then it must have no visible entities
        if !directional_light.shadows_enabled || !light_view_visibility.get() {
            continue;
        }

        let view_mask = maybe_view_mask.unwrap_or_default();

        for (view, view_frusta) in &frusta.frusta {
            visible_entity_query.par_iter().for_each_init(
                || {
                    let mut entities = view_visible_entities_queue.borrow_local_mut();
                    entities.resize(view_frusta.len(), Vec::default());
                    (defer_visible_entities_queue.borrow_local_mut(), entities)
                },
                |(defer_visible_entities_local_queue, view_visible_entities_local_queue),
                 (
                    entity,
                    inherited_visibility,
                    maybe_entity_mask,
                    maybe_aabb,
                    maybe_transform,
                    has_visibility_range,
                    has_no_frustum_culling,
                )| {
                    if !inherited_visibility.get() {
                        return;
                    }

                    let entity_mask = maybe_entity_mask.unwrap_or_default();
                    if !view_mask.intersects(entity_mask) {
                        return;
                    }

                    // Check visibility ranges.
                    if has_visibility_range
                        && visible_entity_ranges.is_some_and(|visible_entity_ranges| {
                            !visible_entity_ranges.entity_is_in_range_of_view(entity, *view)
                        })
                    {
                        return;
                    }

                    if let (Some(aabb), Some(transform)) = (maybe_aabb, maybe_transform) {
                        let mut visible = false;
                        for (frustum, frustum_visible_entities) in view_frusta
                            .iter()
                            .zip(view_visible_entities_local_queue.iter_mut())
                        {
                            // Disable near-plane culling, as a shadow caster could lie before the near plane.
                            if !has_no_frustum_culling
                                && !frustum.intersects_obb(aabb, &transform.affine(), false, true)
                            {
                                continue;
                            }
                            visible = true;

                            frustum_visible_entities.push(entity);
                        }
                        if visible {
                            defer_visible_entities_local_queue.push(entity);
                        }
                    } else {
                        defer_visible_entities_local_queue.push(entity);
                        for frustum_visible_entities in view_visible_entities_local_queue.iter_mut()
                        {
                            frustum_visible_entities.push(entity);
                        }
                    }
                },
            );
            // collect entities from parallel queue
            for entities in view_visible_entities_queue.iter_mut() {
                visible_entities
                    .entities
                    .get_mut(view)
                    .unwrap()
                    .iter_mut()
                    .zip(entities.iter_mut())
                    .for_each(|(dst, source)| {
                        dst.append(source);
                    });
            }
        }

        for (_, cascade_view_entities) in &mut visible_entities.entities {
            cascade_view_entities
                .iter_mut()
                .map(DerefMut::deref_mut)
                .for_each(shrink_entities);
        }
    }

    // Defer marking view visibility so this system can run in parallel with check_point_light_mesh_visibility
    // TODO: use resource to avoid unnecessary memory alloc
    let mut defer_queue = core::mem::take(defer_visible_entities_queue.deref_mut());
    commands.queue(move |world: &mut World| {
        world.resource_scope::<PreviousVisibleEntities, _>(
            |world, mut previous_visible_entities| {
                let mut query = world.query::<(Entity, &mut ViewVisibility)>();
                for entities in defer_queue.iter_mut() {
                    let mut iter = query.iter_many_mut(world, entities.iter());
                    while let Some((entity, mut view_visibility)) = iter.fetch_next() {
                        if !**view_visibility {
                            view_visibility.set();
                        }

                        // Remove any entities that were discovered to be
                        // visible from the `PreviousVisibleEntities` resource.
                        previous_visible_entities.remove(&entity);
                    }
                }
            },
        );
    });
}

pub fn check_point_light_mesh_visibility(
    visible_point_lights: Query<&VisibleClusterableObjects>,
    mut point_lights: Query<(
        &PointLight,
        &GlobalTransform,
        &CubemapFrusta,
        &mut CubemapVisibleEntities,
        Option<&RenderLayers>,
    )>,
    mut spot_lights: Query<(
        &SpotLight,
        &GlobalTransform,
        &Frustum,
        &mut VisibleMeshEntities,
        Option<&RenderLayers>,
    )>,
    mut visible_entity_query: Query<
        (
            Entity,
            &InheritedVisibility,
            &mut ViewVisibility,
            Option<&RenderLayers>,
            Option<&Aabb>,
            Option<&GlobalTransform>,
            Has<VisibilityRange>,
            Has<NoFrustumCulling>,
        ),
        (
            Without<NotShadowCaster>,
            Without<DirectionalLight>,
            With<Mesh3d>,
        ),
    >,
    visible_entity_ranges: Option<Res<VisibleEntityRanges>>,
    mut previous_visible_entities: ResMut<PreviousVisibleEntities>,
    mut cubemap_visible_entities_queue: Local<Parallel<[Vec<Entity>; 6]>>,
    mut spot_visible_entities_queue: Local<Parallel<Vec<Entity>>>,
    mut checked_lights: Local<EntityHashSet>,
) {
    checked_lights.clear();

    let visible_entity_ranges = visible_entity_ranges.as_deref();
    for visible_lights in &visible_point_lights {
        for light_entity in visible_lights.entities.iter().copied() {
            if !checked_lights.insert(light_entity) {
                continue;
            }

            // Point lights
            if let Ok((
                point_light,
                transform,
                cubemap_frusta,
                mut cubemap_visible_entities,
                maybe_view_mask,
            )) = point_lights.get_mut(light_entity)
            {
                for visible_entities in cubemap_visible_entities.iter_mut() {
                    visible_entities.entities.clear();
                }

                // NOTE: If shadow mapping is disabled for the light then it must have no visible entities
                if !point_light.shadows_enabled {
                    continue;
                }

                let view_mask = maybe_view_mask.unwrap_or_default();
                let light_sphere = Sphere {
                    center: Vec3A::from(transform.translation()),
                    radius: point_light.range,
                };

                visible_entity_query.par_iter_mut().for_each_init(
                    || cubemap_visible_entities_queue.borrow_local_mut(),
                    |cubemap_visible_entities_local_queue,
                     (
                        entity,
                        inherited_visibility,
                        mut view_visibility,
                        maybe_entity_mask,
                        maybe_aabb,
                        maybe_transform,
                        has_visibility_range,
                        has_no_frustum_culling,
                    )| {
                        if !inherited_visibility.get() {
                            return;
                        }
                        let entity_mask = maybe_entity_mask.unwrap_or_default();
                        if !view_mask.intersects(entity_mask) {
                            return;
                        }
                        if has_visibility_range
                            && visible_entity_ranges.is_some_and(|visible_entity_ranges| {
                                !visible_entity_ranges.entity_is_in_range_of_any_view(entity)
                            })
                        {
                            return;
                        }

                        // If we have an aabb and transform, do frustum culling
                        if let (Some(aabb), Some(transform)) = (maybe_aabb, maybe_transform) {
                            let model_to_world = transform.affine();
                            // Do a cheap sphere vs obb test to prune out most meshes outside the sphere of the light
                            if !has_no_frustum_culling
                                && !light_sphere.intersects_obb(aabb, &model_to_world)
                            {
                                return;
                            }

                            for (frustum, visible_entities) in cubemap_frusta
                                .iter()
                                .zip(cubemap_visible_entities_local_queue.iter_mut())
                            {
                                if has_no_frustum_culling
                                    || frustum.intersects_obb(aabb, &model_to_world, true, true)
                                {
                                    if !**view_visibility {
                                        view_visibility.set();
                                    }
                                    visible_entities.push(entity);
                                }
                            }
                        } else {
                            if !**view_visibility {
                                view_visibility.set();
                            }
                            for visible_entities in cubemap_visible_entities_local_queue.iter_mut()
                            {
                                visible_entities.push(entity);
                            }
                        }
                    },
                );

                for entities in cubemap_visible_entities_queue.iter_mut() {
                    for (dst, source) in
                        cubemap_visible_entities.iter_mut().zip(entities.iter_mut())
                    {
                        // Remove any entities that were discovered to be
                        // visible from the `PreviousVisibleEntities` resource.
                        for entity in source.iter() {
                            previous_visible_entities.remove(entity);
                        }

                        dst.entities.append(source);
                    }
                }

                for visible_entities in cubemap_visible_entities.iter_mut() {
                    shrink_entities(visible_entities);
                }
            }

            // Spot lights
            if let Ok((point_light, transform, frustum, mut visible_entities, maybe_view_mask)) =
                spot_lights.get_mut(light_entity)
            {
                visible_entities.clear();

                // NOTE: If shadow mapping is disabled for the light then it must have no visible entities
                if !point_light.shadows_enabled {
                    continue;
                }

                let view_mask = maybe_view_mask.unwrap_or_default();
                let light_sphere = Sphere {
                    center: Vec3A::from(transform.translation()),
                    radius: point_light.range,
                };

                visible_entity_query.par_iter_mut().for_each_init(
                    || spot_visible_entities_queue.borrow_local_mut(),
                    |spot_visible_entities_local_queue,
                     (
                        entity,
                        inherited_visibility,
                        mut view_visibility,
                        maybe_entity_mask,
                        maybe_aabb,
                        maybe_transform,
                        has_visibility_range,
                        has_no_frustum_culling,
                    )| {
                        if !inherited_visibility.get() {
                            return;
                        }

                        let entity_mask = maybe_entity_mask.unwrap_or_default();
                        if !view_mask.intersects(entity_mask) {
                            return;
                        }
                        // Check visibility ranges.
                        if has_visibility_range
                            && visible_entity_ranges.is_some_and(|visible_entity_ranges| {
                                !visible_entity_ranges.entity_is_in_range_of_any_view(entity)
                            })
                        {
                            return;
                        }

                        if let (Some(aabb), Some(transform)) = (maybe_aabb, maybe_transform) {
                            let model_to_world = transform.affine();
                            // Do a cheap sphere vs obb test to prune out most meshes outside the sphere of the light
                            if !has_no_frustum_culling
                                && !light_sphere.intersects_obb(aabb, &model_to_world)
                            {
                                return;
                            }

                            if has_no_frustum_culling
                                || frustum.intersects_obb(aabb, &model_to_world, true, true)
                            {
                                if !**view_visibility {
                                    view_visibility.set();
                                }
                                spot_visible_entities_local_queue.push(entity);
                            }
                        } else {
                            if !**view_visibility {
                                view_visibility.set();
                            }
                            spot_visible_entities_local_queue.push(entity);
                        }
                    },
                );

                for entities in spot_visible_entities_queue.iter_mut() {
                    visible_entities.append(entities);

                    // Remove any entities that were discovered to be visible
                    // from the `PreviousVisibleEntities` resource.
                    for entity in entities {
                        previous_visible_entities.remove(entity);
                    }
                }

                shrink_entities(visible_entities.deref_mut());
            }
        }
    }
}
