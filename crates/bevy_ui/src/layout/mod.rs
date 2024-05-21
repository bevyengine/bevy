use thiserror::Error;

use crate::{ContentSize, DefaultUiCamera, Node, Outline, Style, TargetCamera, UiScale};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    entity::Entity,
    event::EventReader,
    query::{With, Without},
    removal_detection::RemovedComponents,
    system::{Query, Res, ResMut, SystemParam},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_math::{UVec2, Vec2};
use bevy_render::camera::{Camera, NormalizedRenderTarget};
use bevy_transform::components::Transform;
use bevy_utils::tracing::warn;
use bevy_utils::{HashMap, HashSet};
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};
use ui_surface::UiSurface;

mod convert;
pub mod debug;
pub(crate) mod ui_surface;

pub struct LayoutContext {
    pub scale_factor: f32,
    pub physical_size: Vec2,
    pub min_size: f32,
    pub max_size: f32,
}

impl LayoutContext {
    pub const DEFAULT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::ZERO,
        min_size: 0.0,
        max_size: 0.0,
    };
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    fn new(scale_factor: f32, physical_size: Vec2) -> Self {
        Self {
            scale_factor,
            physical_size,
            min_size: physical_size.x.min(physical_size.y),
            max_size: physical_size.x.max(physical_size.y),
        }
    }
}

impl Default for LayoutContext {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("Invalid hierarchy")]
    InvalidHierarchy,
    #[error("Taffy error: {0}")]
    TaffyError(#[from] taffy::TaffyError),
}

#[derive(SystemParam)]
pub struct UiLayoutSystemRemovedComponentParam<'w, 's> {
    removed_cameras: RemovedComponents<'w, 's, Camera>,
    removed_children: RemovedComponents<'w, 's, Children>,
    removed_content_sizes: RemovedComponents<'w, 's, ContentSize>,
    removed_nodes: RemovedComponents<'w, 's, Node>,
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
#[allow(clippy::too_many_arguments)]
pub fn ui_layout_system(
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    cameras: Query<(Entity, &Camera)>,
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
    mut resize_events: EventReader<bevy_window::WindowResized>,
    mut ui_surface: ResMut<UiSurface>,
    root_node_query: Query<(Entity, Option<&TargetCamera>), (With<Node>, Without<Parent>)>,
    mut style_query: Query<
        (
            Entity,
            Ref<Style>,
            Option<&mut ContentSize>,
            Option<&TargetCamera>,
        ),
        With<Node>,
    >,
    children_query: Query<(Entity, Ref<Children>), With<Node>>,
    just_children_query: Query<&Children>,
    mut removed_components: UiLayoutSystemRemovedComponentParam,
    mut node_transform_query: Query<(&mut Node, &mut Transform)>,
) {
    struct CameraLayoutInfo {
        size: UVec2,
        resized: bool,
        scale_factor: f32,
        root_nodes: Vec<Entity>,
    }

    let camera_with_default = |target_camera: Option<&TargetCamera>| {
        target_camera
            .map(TargetCamera::entity)
            .or(default_ui_camera.get())
    };

    let resized_windows: HashSet<Entity> = resize_events.read().map(|event| event.window).collect();
    let calculate_camera_layout_info = |camera: &Camera| {
        let size = camera.physical_viewport_size().unwrap_or(UVec2::ZERO);
        let scale_factor = camera.target_scaling_factor().unwrap_or(1.0);
        let camera_target = camera
            .target
            .normalize(primary_window.get_single().map(|(e, _)| e).ok());
        let resized = matches!(camera_target,
          Some(NormalizedRenderTarget::Window(window_ref)) if resized_windows.contains(&window_ref.entity())
        );
        CameraLayoutInfo {
            size,
            resized,
            scale_factor: scale_factor * ui_scale.0,
            root_nodes: Vec::new(),
        }
    };

    // Precalculate the layout info for each camera, so we have fast access to it for each node
    let mut camera_layout_info: HashMap<Entity, CameraLayoutInfo> = HashMap::new();
    for (entity, target_camera) in &root_node_query {
        match camera_with_default(target_camera) {
            Some(camera_entity) => {
                let Ok((_, camera)) = cameras.get(camera_entity) else {
                    warn!(
                        "TargetCamera (of root UI node {entity:?}) is pointing to a camera {:?} which doesn't exist",
                        camera_entity
                    );
                    continue;
                };
                let layout_info = camera_layout_info
                    .entry(camera_entity)
                    .or_insert_with(|| calculate_camera_layout_info(camera));
                layout_info.root_nodes.push(entity);
            }
            None => {
                if cameras.is_empty() {
                    warn!("No camera found to render UI to. To fix this, add at least one camera to the scene.");
                } else {
                    warn!(
                        "Multiple cameras found, causing UI target ambiguity. \
                        To fix this, add an explicit `TargetCamera` component to the root UI node {:?}",
                        entity
                    );
                }
                continue;
            }
        }
    }

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_components.removed_content_sizes.read() {
        ui_surface.try_remove_node_context(entity);
    }

    // Sync Style and ContentSize to Taffy for all nodes
    for (entity, style, content_size, target_camera) in style_query.iter_mut() {
        if let Some(camera) =
            camera_with_default(target_camera).and_then(|c| camera_layout_info.get(&c))
        {
            if camera.resized
                || !scale_factor_events.is_empty()
                || ui_scale.is_changed()
                || style.is_changed()
                || content_size
                    .as_ref()
                    .map(|c| c.measure.is_some())
                    .unwrap_or(false)
            {
                let layout_context = LayoutContext::new(
                    camera.scale_factor,
                    [camera.size.x as f32, camera.size.y as f32].into(),
                );
                let measure = content_size.and_then(|mut c| c.measure.take());
                ui_surface.upsert_node(&layout_context, entity, &style, measure);
            }
        } else {
            ui_surface.upsert_node(&LayoutContext::DEFAULT, entity, &Style::default(), None);
        }
    }
    scale_factor_events.clear();

    // clean up removed nodes
    ui_surface.remove_entities(removed_components.removed_nodes.read());

    // clean up removed cameras
    ui_surface.remove_camera_entities(removed_components.removed_cameras.read());

    // update camera children
    for (camera_id, _) in cameras.iter() {
        let root_nodes =
            if let Some(CameraLayoutInfo { root_nodes, .. }) = camera_layout_info.get(&camera_id) {
                root_nodes.iter().cloned()
            } else {
                [].iter().cloned()
            };
        ui_surface.set_camera_children(camera_id, root_nodes);
    }

    // update and remove children
    for entity in removed_components.removed_children.read() {
        ui_surface.try_remove_children(entity);
    }
    for (entity, children) in &children_query {
        if children.is_changed() {
            ui_surface.update_children(entity, &children);
        }
    }

    for (camera_id, camera) in &camera_layout_info {
        let inverse_target_scale_factor = camera.scale_factor.recip();

        ui_surface.compute_camera_layout(*camera_id, camera.size);
        for root in &camera.root_nodes {
            update_uinode_geometry_recursive(
                *root,
                &ui_surface,
                &mut node_transform_query,
                &just_children_query,
                inverse_target_scale_factor,
                Vec2::ZERO,
                Vec2::ZERO,
            );
        }
    }

    fn update_uinode_geometry_recursive(
        entity: Entity,
        ui_surface: &UiSurface,
        node_transform_query: &mut Query<(&mut Node, &mut Transform)>,
        children_query: &Query<&Children>,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
        mut absolute_location: Vec2,
    ) {
        if let Ok((mut node, mut transform)) = node_transform_query.get_mut(entity) {
            let Ok(layout) = ui_surface.get_layout(entity) else {
                return;
            };
            let layout_size =
                inverse_target_scale_factor * Vec2::new(layout.size.width, layout.size.height);
            let layout_location =
                inverse_target_scale_factor * Vec2::new(layout.location.x, layout.location.y);

            absolute_location += layout_location;

            let rounded_size = round_layout_coords(absolute_location + layout_size)
                - round_layout_coords(absolute_location);

            let rounded_location =
                round_layout_coords(layout_location) + 0.5 * (rounded_size - parent_size);

            // only trigger change detection when the new values are different
            if node.calculated_size != rounded_size || node.unrounded_size != layout_size {
                node.calculated_size = rounded_size;
                node.unrounded_size = layout_size;
            }
            if transform.translation.truncate() != rounded_location {
                transform.translation = rounded_location.extend(0.);
            }
            if let Ok(children) = children_query.get(entity) {
                for &child_uinode in children {
                    update_uinode_geometry_recursive(
                        child_uinode,
                        ui_surface,
                        node_transform_query,
                        children_query,
                        inverse_target_scale_factor,
                        rounded_size,
                        absolute_location,
                    );
                }
            }
        }
    }
}

/// Resolve and update the widths of Node outlines
pub fn resolve_outlines_system(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut outlines_query: Query<(&Outline, &mut Node)>,
) {
    let viewport_size = primary_window
        .get_single()
        .map(|window| window.size())
        .unwrap_or(Vec2::ZERO)
        / ui_scale.0;

    for (outline, mut node) in outlines_query.iter_mut() {
        let node = node.bypass_change_detection();
        node.outline_width = outline
            .width
            .resolve(node.size().x, viewport_size)
            .unwrap_or(0.)
            .max(0.);

        node.outline_offset = outline
            .offset
            .resolve(node.size().x, viewport_size)
            .unwrap_or(0.)
            .max(0.);
    }
}

#[inline]
/// Round `value` to the nearest whole integer, with ties (values with a fractional part equal to 0.5) rounded towards positive infinity.
fn round_ties_up(value: f32) -> f32 {
    if value.fract() != -0.5 {
        // The `round` function rounds ties away from zero. For positive numbers "away from zero" is towards positive infinity.
        // So for all positive values, and negative values with a fractional part not equal to 0.5, `round` returns the correct result.
        value.round()
    } else {
        // In the remaining cases, where `value` is negative and its fractional part is equal to 0.5, we use `ceil` to round it up towards positive infinity.
        value.ceil()
    }
}

#[inline]
/// Rounds layout coordinates by rounding ties upwards.
///
/// Rounding ties up avoids gaining a pixel when rounding bounds that span from negative to positive.
///
/// Example: The width between bounds of -50.5 and 49.5 before rounding is 100, using:
/// - `f32::round`: width becomes 101 (rounds to -51 and 50).
/// - `round_ties_up`: width is 100 (rounds to -50 and 50).
fn round_layout_coords(value: Vec2) -> Vec2 {
    Vec2 {
        x: round_ties_up(value.x),
        y: round_ties_up(value.y),
    }
}

#[cfg(test)]
mod tests {
    use taffy::TraversePartialTree;

    use bevy_asset::AssetEvent;
    use bevy_asset::Assets;
    use bevy_core_pipeline::core_2d::Camera2dBundle;
    use bevy_ecs::entity::Entity;
    use bevy_ecs::event::Events;
    use bevy_ecs::prelude::{Commands, Component, In, Query, With};
    use bevy_ecs::query::Without;
    use bevy_ecs::schedule::apply_deferred;
    use bevy_ecs::schedule::IntoSystemConfigs;
    use bevy_ecs::schedule::Schedule;
    use bevy_ecs::system::RunSystemOnce;
    use bevy_ecs::world::World;
    use bevy_hierarchy::{despawn_with_children_recursive, BuildWorldChildren, Children, Parent};
    use bevy_math::{vec2, Rect, UVec2, Vec2};
    use bevy_render::camera::ManualTextureViews;
    use bevy_render::camera::OrthographicProjection;
    use bevy_render::prelude::Camera;
    use bevy_render::texture::Image;
    use bevy_transform::prelude::GlobalTransform;
    use bevy_transform::systems::{propagate_transforms, sync_simple_transforms};
    use bevy_utils::prelude::default;
    use bevy_utils::HashMap;
    use bevy_window::PrimaryWindow;
    use bevy_window::Window;
    use bevy_window::WindowCreated;
    use bevy_window::WindowResized;
    use bevy_window::WindowResolution;
    use bevy_window::WindowScaleFactorChanged;

    use crate::layout::round_layout_coords;
    use crate::layout::ui_surface::UiSurface;
    use crate::prelude::*;
    use crate::ui_layout_system;
    use crate::update::update_target_camera_system;
    use crate::ContentSize;

    #[test]
    fn round_layout_coords_must_round_ties_up() {
        assert_eq!(round_layout_coords(vec2(-50.5, 49.5)), vec2(-50., 50.));
    }

    // these window dimensions are easy to convert to and from percentage values
    const WINDOW_WIDTH: f32 = 1000.;
    const WINDOW_HEIGHT: f32 = 100.;

    fn setup_ui_test_world() -> (World, Schedule) {
        let mut world = World::new();
        world.init_resource::<UiScale>();
        world.init_resource::<UiSurface>();
        world.init_resource::<Events<WindowScaleFactorChanged>>();
        world.init_resource::<Events<WindowResized>>();
        // Required for the camera system
        world.init_resource::<Events<WindowCreated>>();
        world.init_resource::<Events<AssetEvent<Image>>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<ManualTextureViews>();

        // spawn a dummy primary window and camera
        world.spawn((
            Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                ..default()
            },
            PrimaryWindow,
        ));
        world.spawn(Camera2dBundle::default());

        let mut ui_schedule = Schedule::default();
        ui_schedule.add_systems(
            (
                // UI is driven by calculated camera target info, so we need to run the camera system first
                bevy_render::camera::camera_system::<OrthographicProjection>,
                update_target_camera_system,
                apply_deferred,
                ui_layout_system,
                sync_simple_transforms,
                propagate_transforms,
            )
                .chain(),
        );

        (world, ui_schedule)
    }

    #[test]
    fn ui_nodes_with_percent_100_dimensions_should_fill_their_parent() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        // spawn a root entity with width and height set to fill 100% of its parent
        let ui_root = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                ..default()
            })
            .id();

        let ui_child = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                ..default()
            })
            .id();

        world.entity_mut(ui_root).add_child(ui_child);

        ui_schedule.run(&mut world);
        let ui_surface = world.resource::<UiSurface>();

        for ui_entity in [ui_root, ui_child] {
            let layout = ui_surface.get_layout(ui_entity).unwrap();
            assert_eq!(layout.size.width, WINDOW_WIDTH);
            assert_eq!(layout.size.height, WINDOW_HEIGHT);
        }
    }

    #[test]
    fn ui_surface_tracks_ui_entities() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        ui_schedule.run(&mut world);

        // no UI entities in world, none in UiSurface
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.is_empty());

        let ui_entity = world.spawn(NodeBundle::default()).id();

        // `ui_layout_system` should map `ui_entity` to a ui node in `UiSurface::entity_to_taffy`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert_eq!(ui_surface.entity_to_taffy.len(), 1);

        world.despawn(ui_entity);

        // `ui_layout_system` should remove `ui_entity` from `UiSurface::entity_to_taffy`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert!(!ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert!(ui_surface.entity_to_taffy.is_empty());
    }

    #[test]
    fn ui_surface_tracks_camera_entities() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        // despawn all cameras so we can reset ui_surface back to a fresh state
        let camera_entities = world
            .query_filtered::<Entity, With<Camera>>()
            .iter(&world)
            .collect::<Vec<_>>();
        for camera_entity in camera_entities {
            world.despawn(camera_entity);
        }

        ui_schedule.run(&mut world);

        // no UI entities in world, none in UiSurface
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.camera_entity_to_taffy.is_empty());

        // respawn camera
        let camera_entity = world.spawn(Camera2dBundle::default()).id();

        let ui_entity = world
            .spawn((NodeBundle::default(), TargetCamera(camera_entity)))
            .id();

        // `ui_layout_system` should map `camera_entity` to a ui node in `UiSurface::camera_entity_to_taffy`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface
            .camera_entity_to_taffy
            .contains_key(&camera_entity));
        assert_eq!(ui_surface.camera_entity_to_taffy.len(), 1);

        world.despawn(ui_entity);
        world.despawn(camera_entity);

        // `ui_layout_system` should remove `camera_entity` from `UiSurface::camera_entity_to_taffy`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert!(!ui_surface
            .camera_entity_to_taffy
            .contains_key(&camera_entity));
        assert!(ui_surface.camera_entity_to_taffy.is_empty());
    }

    #[test]
    #[should_panic]
    fn despawning_a_ui_entity_should_remove_its_corresponding_ui_node() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let ui_entity = world.spawn(NodeBundle::default()).id();

        // `ui_layout_system` will insert a ui node into the internal layout tree corresponding to `ui_entity`
        ui_schedule.run(&mut world);

        // retrieve the ui node corresponding to `ui_entity` from ui surface
        let ui_surface = world.resource::<UiSurface>();
        let ui_node = ui_surface.entity_to_taffy[&ui_entity];

        world.despawn(ui_entity);

        // `ui_layout_system` will receive a `RemovedComponents<Node>` event for `ui_entity`
        // and remove `ui_entity` from `ui_node` from the internal layout tree
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();

        // `ui_node` is removed, attempting to retrieve a style for `ui_node` panics
        let _ = ui_surface.taffy.style(ui_node);
    }

    #[test]
    fn changes_to_children_of_a_ui_entity_change_its_corresponding_ui_nodes_children() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let ui_parent_entity = world.spawn(NodeBundle::default()).id();

        // `ui_layout_system` will insert a ui node into the internal layout tree corresponding to `ui_entity`
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        let ui_parent_node = ui_surface.entity_to_taffy[&ui_parent_entity];

        // `ui_parent_node` shouldn't have any children yet
        assert_eq!(ui_surface.taffy.child_count(ui_parent_node), 0);

        let mut ui_child_entities = (0..10)
            .map(|_| {
                let child = world.spawn(NodeBundle::default()).id();
                world.entity_mut(ui_parent_entity).add_child(child);
                child
            })
            .collect::<Vec<_>>();

        ui_schedule.run(&mut world);

        // `ui_parent_node` should have children now
        let ui_surface = world.resource::<UiSurface>();
        assert_eq!(
            ui_surface.entity_to_taffy.len(),
            1 + ui_child_entities.len()
        );
        assert_eq!(
            ui_surface.taffy.child_count(ui_parent_node),
            ui_child_entities.len()
        );

        let child_node_map = HashMap::from_iter(
            ui_child_entities
                .iter()
                .map(|child_entity| (*child_entity, ui_surface.entity_to_taffy[child_entity])),
        );

        // the children should have a corresponding ui node and that ui node's parent should be `ui_parent_node`
        for node in child_node_map.values() {
            assert_eq!(ui_surface.taffy.parent(*node), Some(ui_parent_node));
        }

        // delete every second child
        let mut deleted_children = vec![];
        for i in (0..ui_child_entities.len()).rev().step_by(2) {
            let child = ui_child_entities.remove(i);
            world.despawn(child);
            deleted_children.push(child);
        }

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        assert_eq!(
            ui_surface.entity_to_taffy.len(),
            1 + ui_child_entities.len()
        );
        assert_eq!(
            ui_surface.taffy.child_count(ui_parent_node),
            ui_child_entities.len()
        );

        // the remaining children should still have nodes in the layout tree
        for child_entity in &ui_child_entities {
            let child_node = child_node_map[child_entity];
            assert_eq!(ui_surface.entity_to_taffy[child_entity], child_node);
            assert_eq!(ui_surface.taffy.parent(child_node), Some(ui_parent_node));
            assert!(ui_surface
                .taffy
                .children(ui_parent_node)
                .unwrap()
                .contains(&child_node));
        }

        // the nodes of the deleted children should have been removed from the layout tree
        for deleted_child_entity in &deleted_children {
            assert!(!ui_surface
                .entity_to_taffy
                .contains_key(deleted_child_entity));
            let deleted_child_node = child_node_map[deleted_child_entity];
            assert!(!ui_surface
                .taffy
                .children(ui_parent_node)
                .unwrap()
                .contains(&deleted_child_node));
        }

        // despawn the parent entity and its descendants
        despawn_with_children_recursive(&mut world, ui_parent_entity);

        ui_schedule.run(&mut world);

        // all nodes should have been deleted
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.is_empty());
    }

    /// regression test for >=0.13.1 root node layouts
    /// ensure root nodes act like they are absolutely positioned
    /// without explicitly declaring it.
    #[test]
    fn ui_root_node_should_act_like_position_absolute() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let mut size = 150.;

        world.spawn(NodeBundle {
            style: Style {
                // test should pass without explicitly requiring position_type to be set to Absolute
                // position_type: PositionType::Absolute,
                width: Val::Px(size),
                height: Val::Px(size),
                ..default()
            },
            ..default()
        });

        size -= 50.;

        world.spawn(NodeBundle {
            style: Style {
                // position_type: PositionType::Absolute,
                width: Val::Px(size),
                height: Val::Px(size),
                ..default()
            },
            ..default()
        });

        size -= 50.;

        world.spawn(NodeBundle {
            style: Style {
                // position_type: PositionType::Absolute,
                width: Val::Px(size),
                height: Val::Px(size),
                ..default()
            },
            ..default()
        });

        ui_schedule.run(&mut world);

        let overlap_check = world
            .query_filtered::<(Entity, &Node, &GlobalTransform), Without<Parent>>()
            .iter(&world)
            .fold(
                Option::<(Rect, bool)>::None,
                |option_rect, (entity, node, global_transform)| {
                    let current_rect = node.logical_rect(global_transform);
                    assert!(
                        current_rect.height().abs() + current_rect.width().abs() > 0.,
                        "root ui node {entity:?} doesn't have a logical size"
                    );
                    assert_ne!(
                        global_transform.affine(),
                        GlobalTransform::default().affine(),
                        "root ui node {entity:?} global transform is not populated"
                    );
                    let Some((rect, is_overlapping)) = option_rect else {
                        return Some((current_rect, false));
                    };
                    if rect.contains(current_rect.center()) {
                        Some((current_rect, true))
                    } else {
                        Some((current_rect, is_overlapping))
                    }
                },
            );

        let Some((_rect, is_overlapping)) = overlap_check else {
            unreachable!("test not setup properly");
        };
        assert!(is_overlapping, "root ui nodes are expected to behave like they have absolute position and be independent from each other");
    }

    #[test]
    fn ui_node_should_properly_update_when_changing_target_camera() {
        #[derive(Component)]
        struct MovingUiNode;

        fn update_camera_viewports(
            primary_window_query: Query<&Window, With<PrimaryWindow>>,
            mut cameras: Query<&mut Camera>,
        ) {
            let primary_window = primary_window_query
                .get_single()
                .expect("missing primary window");
            let camera_count = cameras.iter().len();
            for (camera_index, mut camera) in cameras.iter_mut().enumerate() {
                let viewport_width =
                    primary_window.resolution.physical_width() / camera_count as u32;
                let viewport_height = primary_window.resolution.physical_height();
                let physical_position = UVec2::new(viewport_width * camera_index as u32, 0);
                let physical_size = UVec2::new(viewport_width, viewport_height);
                camera.viewport = Some(bevy_render::camera::Viewport {
                    physical_position,
                    physical_size,
                    ..default()
                });
            }
        }

        fn move_ui_node(
            In(pos): In<Vec2>,
            mut commands: Commands,
            cameras: Query<(Entity, &Camera)>,
            moving_ui_query: Query<Entity, With<MovingUiNode>>,
        ) {
            let (target_camera_entity, _) = cameras
                .iter()
                .find(|(_, camera)| {
                    let Some(logical_viewport_rect) = camera.logical_viewport_rect() else {
                        panic!("missing logical viewport")
                    };
                    // make sure cursor is in viewport and that viewport has at least 1px of size
                    logical_viewport_rect.contains(pos)
                        && logical_viewport_rect.max.cmpge(Vec2::splat(0.)).any()
                })
                .expect("cursor position outside of camera viewport");
            for moving_ui_entity in moving_ui_query.iter() {
                commands
                    .entity(moving_ui_entity)
                    .insert(TargetCamera(target_camera_entity))
                    .insert(Style {
                        position_type: PositionType::Absolute,
                        top: Val::Px(pos.y),
                        left: Val::Px(pos.x),
                        ..default()
                    });
            }
        }

        fn do_move_and_test(
            world: &mut World,
            ui_schedule: &mut Schedule,
            new_pos: Vec2,
            expected_camera_entity: &Entity,
        ) {
            world.run_system_once_with(new_pos, move_ui_node);
            ui_schedule.run(world);
            let (ui_node_entity, TargetCamera(target_camera_entity)) = world
                .query_filtered::<(Entity, &TargetCamera), With<MovingUiNode>>()
                .get_single(world)
                .expect("missing MovingUiNode");
            assert_eq!(expected_camera_entity, target_camera_entity);
            let ui_surface = world.resource::<UiSurface>();

            let layout = ui_surface
                .get_layout(ui_node_entity)
                .expect("failed to get layout");

            // negative test for #12255
            assert_eq!(Vec2::new(layout.location.x, layout.location.y), new_pos);
        }

        fn get_taffy_node_count(world: &World) -> usize {
            world.resource::<UiSurface>().taffy.total_node_count()
        }

        let (mut world, mut ui_schedule) = setup_ui_test_world();

        world.spawn(Camera2dBundle {
            camera: Camera {
                order: 1,
                ..default()
            },
            ..default()
        });

        world.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.),
                    left: Val::Px(0.),
                    ..default()
                },
                ..default()
            },
            MovingUiNode,
        ));

        ui_schedule.run(&mut world);

        let pos_inc = Vec2::splat(1.);
        let total_cameras = world.query::<&Camera>().iter(&world).len();
        // add total cameras - 1 (the assumed default) to get an idea for how many nodes we should expect
        let expected_max_taffy_node_count = get_taffy_node_count(&world) + total_cameras - 1;

        world.run_system_once(update_camera_viewports);

        ui_schedule.run(&mut world);

        let viewport_rects = world
            .query::<(Entity, &Camera)>()
            .iter(&world)
            .map(|(e, c)| (e, c.logical_viewport_rect().expect("missing viewport")))
            .collect::<Vec<_>>();

        for (camera_entity, viewport) in viewport_rects.iter() {
            let target_pos = viewport.min + pos_inc;
            do_move_and_test(&mut world, &mut ui_schedule, target_pos, camera_entity);
        }

        // reverse direction
        let mut viewport_rects = viewport_rects.clone();
        viewport_rects.reverse();
        for (camera_entity, viewport) in viewport_rects.iter() {
            let target_pos = viewport.max - pos_inc;
            do_move_and_test(&mut world, &mut ui_schedule, target_pos, camera_entity);
        }

        let current_taffy_node_count = get_taffy_node_count(&world);
        if current_taffy_node_count > expected_max_taffy_node_count {
            panic!("extra taffy nodes detected: current: {current_taffy_node_count} max expected: {expected_max_taffy_node_count}");
        }
    }

    #[test]
    fn ui_node_should_be_set_to_its_content_size() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let content_size = Vec2::new(50., 25.);

        let ui_entity = world
            .spawn((
                NodeBundle {
                    style: Style {
                        align_self: AlignSelf::Start,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ContentSize::fixed_size(content_size),
            ))
            .id();

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        let layout = ui_surface.get_layout(ui_entity).unwrap();

        // the node should takes its size from the fixed size measure func
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);
    }

    #[test]
    fn measure_funcs_should_be_removed_on_content_size_removal() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let content_size = Vec2::new(50., 25.);
        let ui_entity = world
            .spawn((
                NodeBundle {
                    style: Style {
                        align_self: AlignSelf::Start,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ContentSize::fixed_size(content_size),
            ))
            .id();

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        let ui_node = ui_surface.entity_to_taffy[&ui_entity];

        // a node with a content size should have taffy context
        assert!(ui_surface.taffy.get_node_context(ui_node).is_some());
        let layout = ui_surface.get_layout(ui_entity).unwrap();
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);

        world.entity_mut(ui_entity).remove::<ContentSize>();

        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();
        // a node without a content size should not have taffy context
        assert!(ui_surface.taffy.get_node_context(ui_node).is_none());

        // Without a content size, the node has no width or height constraints so the length of both dimensions is 0.
        let layout = ui_surface.get_layout(ui_entity).unwrap();
        assert_eq!(layout.size.width, 0.);
        assert_eq!(layout.size.height, 0.);
    }

    #[test]
    fn ui_rounding_test() {
        let (mut world, mut ui_schedule) = setup_ui_test_world();

        let parent = world
            .spawn(NodeBundle {
                style: Style {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::min_content(2),
                    margin: UiRect::all(Val::Px(4.0)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|commands| {
                for _ in 0..2 {
                    commands.spawn(NodeBundle {
                        style: Style {
                            display: Display::Grid,
                            width: Val::Px(160.),
                            height: Val::Px(160.),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }
            })
            .id();

        let children = world
            .entity(parent)
            .get::<Children>()
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<Entity>>();

        for r in [2, 3, 5, 7, 11, 13, 17, 19, 21, 23, 29, 31].map(|n| (n as f32).recip()) {
            let mut s = r;
            while s <= 5. {
                world.resource_mut::<UiScale>().0 = s;
                ui_schedule.run(&mut world);
                let width_sum: f32 = children
                    .iter()
                    .map(|child| world.get::<Node>(*child).unwrap().calculated_size.x)
                    .sum();
                let parent_width = world.get::<Node>(parent).unwrap().calculated_size.x;
                assert!((width_sum - parent_width).abs() < 0.001);
                assert!((width_sum - 320.).abs() <= 1.);
                s += r;
            }
        }
    }

    #[test]
    fn no_camera_ui() {
        let mut world = World::new();
        world.init_resource::<UiScale>();
        world.init_resource::<UiSurface>();
        world.init_resource::<Events<WindowScaleFactorChanged>>();
        world.init_resource::<Events<WindowResized>>();
        // Required for the camera system
        world.init_resource::<Events<WindowCreated>>();
        world.init_resource::<Events<AssetEvent<Image>>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<ManualTextureViews>();

        // spawn a dummy primary window and camera
        world.spawn((
            Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                ..default()
            },
            PrimaryWindow,
        ));

        let mut ui_schedule = Schedule::default();
        ui_schedule.add_systems(
            (
                // UI is driven by calculated camera target info, so we need to run the camera system first
                bevy_render::camera::camera_system::<OrthographicProjection>,
                update_target_camera_system,
                apply_deferred,
                ui_layout_system,
            )
                .chain(),
        );

        let ui_root = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                ..default()
            })
            .id();

        let ui_child = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                ..default()
            })
            .id();

        world.entity_mut(ui_root).add_child(ui_child);

        ui_schedule.run(&mut world);
    }
}
