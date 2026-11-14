use bevy_app::{App, PostUpdate};
use bevy_camera::{
    primitives::Aabb,
    visibility::{RenderLayers, VisibilitySystems},
};
use bevy_ecs::{
    hierarchy::Children,
    schedule::IntoScheduleConfigs,
    system::{In, IntoSystem, Query, Res, ResMut},
};
use bevy_feathers::palette;
use bevy_gizmos::{
    config::{GizmoConfig, GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder, GizmoPlugin,
};
use bevy_reflect::Reflect;
use bevy_shape::{Aabb3d, BoundingVolume};
use bevy_transform::{components::GlobalTransform, TransformSystems};

use crate::{inspector_shown, InspectorMode, InspectorSelection, InspectorSource};

#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
struct InspectorOutline;

struct Outline {
    aabb: Aabb3d,
    transform: GlobalTransform,
    layers: RenderLayers,
}

pub(crate) fn add_outline(app: &mut App) {
    let registered = app
        .world()
        .get_resource::<GizmoConfigStore>()
        .is_some_and(|store| store.get_config::<InspectorOutline>().is_some());
    if registered || !app.is_plugin_added::<GizmoPlugin>() {
        return;
    }
    app.insert_gizmo_config(
        InspectorOutline,
        GizmoConfig {
            depth_bias: -1.0,
            ..Default::default()
        },
    )
    .add_systems(
        PostUpdate,
        inspector_shown
            .pipe(selection_outline)
            .pipe(set_outline_layers)
            .pipe(draw_outline)
            .ambiguous_with_all()
            .after(TransformSystems::Propagate)
            .after(VisibilitySystems::CalculateBounds),
    );
}

fn selection_outline(
    In(visible): In<bool>,
    mode: Res<InspectorMode>,
    selection: Res<InspectorSelection>,
    bounds: Query<(&Aabb, &GlobalTransform, Option<&RenderLayers>)>,
    children: Query<&Children>,
) -> Option<Outline> {
    if !visible || *mode != InspectorMode::Edit {
        return None;
    }
    let selected = selection
        .0
        .as_ref()
        .filter(|selection| selection.source == InspectorSource::Local)?
        .entity;
    if let Ok((aabb, transform, layers)) = bounds.get(selected) {
        return Some(Outline {
            aabb: Aabb3d::new(aabb.center, aabb.half_extents),
            transform: *transform,
            layers: layers.cloned().unwrap_or_default(),
        });
    }
    children
        .iter_descendants(selected)
        .filter_map(|descendant| bounds.get(descendant).ok())
        .map(|(aabb, transform, layers)| {
            (
                world_aabb(aabb, transform),
                layers.cloned().unwrap_or_default(),
            )
        })
        .reduce(|(union, union_layers), (aabb, layers)| {
            (union.merge(&aabb), union_layers.union(&layers))
        })
        .map(|(aabb, layers)| Outline {
            aabb,
            transform: GlobalTransform::IDENTITY,
            layers,
        })
}

fn world_aabb(aabb: &Aabb, transform: &GlobalTransform) -> Aabb3d {
    let affine = transform.affine();
    let axes = affine.matrix3;
    let half_size = axes.x_axis.abs() * aabb.half_extents.x
        + axes.y_axis.abs() * aabb.half_extents.y
        + axes.z_axis.abs() * aabb.half_extents.z;
    Aabb3d::new(affine.transform_point3a(aabb.center), half_size)
}

fn set_outline_layers(
    In(outline): In<Option<Outline>>,
    mut store: ResMut<GizmoConfigStore>,
) -> Option<Outline> {
    let outline = outline?;
    let (config, _) = store.config_mut::<InspectorOutline>();
    if config.render_layers != outline.layers {
        config.render_layers = outline.layers.clone();
    }
    Some(outline)
}

fn draw_outline(In(outline): In<Option<Outline>>, mut gizmos: Gizmos<InspectorOutline>) {
    if let Some(outline) = outline {
        gizmos.aabb_3d(outline.aabb, outline.transform, palette::ACCENT);
    }
}

#[cfg(test)]
mod tests {
    use bevy_asset::AssetApp;
    use bevy_ecs::{entity::Entity, hierarchy::ChildOf, system::RunSystemOnce, world::World};
    use core::f32::consts::FRAC_PI_2;

    use bevy_asset::Assets;
    use bevy_camera::visibility::InheritedVisibility;
    use bevy_gizmos::{GizmoAsset, GizmoHandles};
    use bevy_math::{Quat, Vec3};
    use bevy_transform::components::Transform;

    use super::*;
    use crate::{entity_tree::InspectorTreeView, InspectableEntity, InspectorSources};

    fn world(visible: bool) -> World {
        let mut world = World::new();
        world.init_resource::<InspectorSelection>();
        world.init_resource::<InspectorSources>();
        world.init_resource::<InspectorMode>();
        if visible {
            world.spawn(InspectorTreeView);
        }
        world
    }

    fn unit_box(world: &mut World, translation: Vec3) -> Entity {
        world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5)),
                GlobalTransform::from_translation(translation),
            ))
            .id()
    }

    fn outline(world: &mut World, selected: Entity) -> Option<Outline> {
        world.resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(selected, InspectorSource::Local));
        world
            .run_system_once(inspector_shown.pipe(selection_outline))
            .unwrap()
    }

    #[test]
    fn an_entity_with_bounds_is_outlined_by_its_own_box() {
        let mut world = world(true);
        let cube = unit_box(&mut world, Vec3::new(1.0, 2.0, 3.0));
        let outline = outline(&mut world, cube).unwrap();
        assert_eq!(outline.aabb.min, Vec3::splat(-0.5).into());
        assert_eq!(outline.aabb.max, Vec3::splat(0.5).into());
        assert_eq!(outline.transform.translation(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(outline.layers, RenderLayers::default());
    }

    #[test]
    fn a_hidden_selection_is_still_outlined() {
        let mut world = world(true);
        let parent = world.spawn(InheritedVisibility::HIDDEN).id();
        let child = unit_box(&mut world, Vec3::ZERO);
        world
            .entity_mut(child)
            .insert((ChildOf(parent), InheritedVisibility::HIDDEN));
        assert!(outline(&mut world, child).is_some());
        assert!(outline(&mut world, parent).is_some());
    }

    #[test]
    fn a_parent_without_bounds_is_outlined_around_its_descendants() {
        let mut world = world(true);
        let parent = world.spawn_empty().id();
        let left = unit_box(&mut world, Vec3::new(-2.0, 0.0, 0.0));
        let middle = world.spawn(ChildOf(parent)).id();
        let right = unit_box(&mut world, Vec3::new(2.0, 1.0, 0.0));
        let turned = unit_box(&mut world, Vec3::new(0.0, 0.0, 3.0));
        world
            .entity_mut(left)
            .insert((ChildOf(parent), RenderLayers::layer(1)));
        world.entity_mut(right).insert(ChildOf(middle));
        world.entity_mut(turned).insert((
            ChildOf(middle),
            GlobalTransform::from(
                Transform::from_xyz(0.0, 0.0, 3.0)
                    .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
                    .with_scale(Vec3::new(2.0, 1.0, 1.0)),
            ),
        ));
        let outline = outline(&mut world, parent).unwrap();
        assert!(outline
            .aabb
            .min
            .abs_diff_eq(Vec3::new(-2.5, -0.5, -0.5).into(), 1e-5));
        assert!(outline
            .aabb
            .max
            .abs_diff_eq(Vec3::new(2.5, 1.5, 4.0).into(), 1e-5));
        assert_eq!(outline.transform, GlobalTransform::IDENTITY);
        assert_eq!(outline.layers, RenderLayers::from_layers(&[0, 1]));
    }

    #[test]
    fn nothing_is_outlined_without_bounds() {
        let mut world = world(true);
        let parent = world.spawn_empty().id();
        world.spawn(ChildOf(parent));
        assert!(outline(&mut world, parent).is_none());
    }

    #[test]
    fn nothing_is_outlined_while_the_tree_is_hidden() {
        let mut world = world(false);
        let cube = unit_box(&mut world, Vec3::ZERO);
        assert!(outline(&mut world, cube).is_none());
    }

    #[test]
    fn nothing_is_outlined_in_preview_mode() {
        let mut world = world(true);
        let cube = unit_box(&mut world, Vec3::ZERO);
        *world.resource_mut::<InspectorMode>() = InspectorMode::Preview;
        assert!(outline(&mut world, cube).is_none());
        *world.resource_mut::<InspectorMode>() = InspectorMode::Edit;
        assert!(outline(&mut world, cube).is_some());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn nothing_is_outlined_for_a_remote_selection() {
        use crate::{remote::RemoteSource, WorldKind};

        let mut world = world(true);
        let cube = unit_box(&mut world, Vec3::ZERO);
        world.resource_mut::<InspectorSelection>().0 = Some(InspectableEntity::new(
            cube,
            InspectorSource::Remote(RemoteSource::new("localhost", 1, WorldKind::Main)),
        ));
        assert!(world
            .run_system_once(inspector_shown.pipe(selection_outline))
            .unwrap()
            .is_none());
    }

    fn plugin_app(gizmos: bool) -> App {
        let mut app = App::new();
        app.add_plugins((
            bevy_app::TaskPoolPlugin::default(),
            bevy_time::TimePlugin,
            bevy_asset::AssetPlugin::default(),
            bevy_scene::ScenePlugin,
            crate::InspectorPlugin,
        ));
        app.init_asset::<bevy_image::Image>()
            .init_asset::<bevy_text::Font>();
        if gizmos {
            app.add_plugins(GizmoPlugin);
        }
        app.finish();
        app.cleanup();
        app
    }

    #[test]
    fn the_outline_draws_on_top_in_its_own_gizmo_group() {
        let mut app = plugin_app(true);
        app.world_mut().spawn(InspectorTreeView);
        let cube = unit_box(app.world_mut(), Vec3::ZERO);
        app.world_mut()
            .entity_mut(cube)
            .insert(RenderLayers::layer(2));
        app.update();
        assert_eq!(outline_vertices(&app), 0);

        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(cube, InspectorSource::Local));
        app.update();
        assert!(outline_vertices(&app) > 0);
        let store = app.world().resource::<GizmoConfigStore>();
        let config = store.config::<InspectorOutline>().0;
        assert_eq!(config.depth_bias, -1.0);
        assert_eq!(config.render_layers, RenderLayers::layer(2));
        assert_eq!(
            store
                .config::<bevy_gizmos::config::DefaultGizmoConfigGroup>()
                .0
                .depth_bias,
            0.0
        );
    }

    fn outline_vertices(app: &App) -> usize {
        app.world()
            .resource::<GizmoHandles>()
            .handles()
            .get(&core::any::TypeId::of::<InspectorOutline>())
            .cloned()
            .flatten()
            .and_then(|handle| app.world().resource::<Assets<GizmoAsset>>().get(&handle))
            .map_or(0, |asset| {
                asset.buffer().list_positions.len() + asset.buffer().strip_positions.len()
            })
    }

    #[test]
    fn apps_without_gizmos_get_no_outline() {
        let mut app = plugin_app(false);
        app.update();
        assert!(app
            .world()
            .get_resource::<GizmoConfigStore>()
            .is_none_or(|store| store.get_config::<InspectorOutline>().is_none()));
    }
}
