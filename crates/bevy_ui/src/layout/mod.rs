use crate::{
    BorderRadius, ContentSize, DefaultUiCamera, Display, Node, Outline, OverflowAxis,
    ScrollPosition, Style, TargetCamera, UiChildren, UiRootNodes, UiScale,
};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    entity::{Entity, EntityHashMap, EntityHashSet},
    event::EventReader,
    query::With,
    removal_detection::RemovedComponents,
    system::{Commands, Local, Query, Res, ResMut, SystemParam},
    world::Ref,
};
use bevy_hierarchy::Children;
use bevy_math::{UVec2, Vec2};
use bevy_render::camera::{Camera, NormalizedRenderTarget};
use bevy_sprite::BorderRect;
use bevy_transform::components::Transform;
use bevy_utils::tracing::warn;
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};
use derive_more::derive::{Display, Error, From};
use ui_surface::UiSurface;

#[cfg(feature = "bevy_text")]
use bevy_text::CosmicBuffer;
#[cfg(feature = "bevy_text")]
use bevy_text::CosmicFontSystem;

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

#[derive(Debug, Error, Display, From)]
pub enum LayoutError {
    #[display("Invalid hierarchy")]
    InvalidHierarchy,
    #[display("Taffy error: {_0}")]
    TaffyError(taffy::TaffyError),
}

#[doc(hidden)]
#[derive(SystemParam)]
pub struct UiLayoutSystemRemovedComponentParam<'w, 's> {
    removed_cameras: RemovedComponents<'w, 's, Camera>,
    removed_children: RemovedComponents<'w, 's, Children>,
    removed_content_sizes: RemovedComponents<'w, 's, ContentSize>,
    removed_nodes: RemovedComponents<'w, 's, Node>,
}

#[doc(hidden)]
#[derive(Default)]
pub struct UiLayoutSystemBuffers {
    interned_root_nodes: Vec<Vec<Entity>>,
    resized_windows: EntityHashSet,
    camera_layout_info: EntityHashMap<CameraLayoutInfo>,
}

struct CameraLayoutInfo {
    size: UVec2,
    resized: bool,
    scale_factor: f32,
    root_nodes: Vec<Entity>,
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
#[allow(clippy::too_many_arguments)]
pub fn ui_layout_system(
    mut commands: Commands,
    mut buffers: Local<UiLayoutSystemBuffers>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    camera_data: (Query<(Entity, &Camera)>, DefaultUiCamera),
    ui_scale: Res<UiScale>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
    mut resize_events: EventReader<bevy_window::WindowResized>,
    mut ui_surface: ResMut<UiSurface>,
    root_nodes: UiRootNodes,
    mut style_query: Query<
        (
            Entity,
            Ref<Style>,
            Option<&mut ContentSize>,
            Option<&TargetCamera>,
        ),
        With<Node>,
    >,
    node_query: Query<Entity, With<Node>>,
    ui_children: UiChildren,
    mut removed_components: UiLayoutSystemRemovedComponentParam,
    mut node_transform_query: Query<(
        &mut Node,
        &mut Transform,
        &Style,
        Option<&BorderRadius>,
        Option<&Outline>,
        Option<&ScrollPosition>,
    )>,
    #[cfg(feature = "bevy_text")] mut buffer_query: Query<&mut CosmicBuffer>,
    #[cfg(feature = "bevy_text")] mut font_system: ResMut<CosmicFontSystem>,
) {
    let UiLayoutSystemBuffers {
        interned_root_nodes,
        resized_windows,
        camera_layout_info,
    } = &mut *buffers;

    let (cameras, default_ui_camera) = camera_data;

    let default_camera = default_ui_camera.get();
    let camera_with_default = |target_camera: Option<&TargetCamera>| {
        target_camera.map(TargetCamera::entity).or(default_camera)
    };

    resized_windows.clear();
    resized_windows.extend(resize_events.read().map(|event| event.window));
    let mut calculate_camera_layout_info = |camera: &Camera| {
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
            root_nodes: interned_root_nodes.pop().unwrap_or_default(),
        }
    };

    // Precalculate the layout info for each camera, so we have fast access to it for each node
    camera_layout_info.clear();

    style_query
        .iter_many(root_nodes.iter())
        .for_each(|(entity, _, _, target_camera)| {
            match camera_with_default(target_camera) {
                Some(camera_entity) => {
                    let Ok((_, camera)) = cameras.get(camera_entity) else {
                        warn!(
                            "TargetCamera (of root UI node {entity:?}) is pointing to a camera {:?} which doesn't exist",
                            camera_entity
                        );
                        return;
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
                }
            }

        }
    );

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_components.removed_content_sizes.read() {
        ui_surface.try_remove_node_context(entity);
    }

    // Sync Style and ContentSize to Taffy for all nodes
    style_query
        .iter_mut()
        .for_each(|(entity, style, content_size, target_camera)| {
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
        });
    scale_factor_events.clear();

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

    node_query.iter().for_each(|entity| {
        if ui_children.is_changed(entity) {
            ui_surface.update_children(entity, ui_children.iter_ui_children(entity));
        }
    });

    #[cfg(feature = "bevy_text")]
    let text_buffers = &mut buffer_query;
    // clean up removed nodes after syncing children to avoid potential panic (invalid SlotMap key used)
    ui_surface.remove_entities(removed_components.removed_nodes.read());

    // Re-sync changed children: avoid layout glitches caused by removed nodes that are still set as a child of another node
    node_query.iter().for_each(|entity| {
        if ui_children.is_changed(entity) {
            ui_surface.update_children(entity, ui_children.iter_ui_children(entity));
        }
    });

    for (camera_id, mut camera) in camera_layout_info.drain() {
        let inverse_target_scale_factor = camera.scale_factor.recip();

        ui_surface.compute_camera_layout(
            camera_id,
            camera.size,
            #[cfg(feature = "bevy_text")]
            text_buffers,
            #[cfg(feature = "bevy_text")]
            &mut font_system.0,
        );

        for root in &camera.root_nodes {
            update_uinode_geometry_recursive(
                &mut commands,
                *root,
                &ui_surface,
                None,
                &mut node_transform_query,
                &ui_children,
                inverse_target_scale_factor,
                Vec2::ZERO,
                Vec2::ZERO,
                Vec2::ZERO,
            );
        }

        camera.root_nodes.clear();
        interned_root_nodes.push(camera.root_nodes);
    }

    // Returns the combined bounding box of the node and any of its overflowing children.
    fn update_uinode_geometry_recursive(
        commands: &mut Commands,
        entity: Entity,
        ui_surface: &UiSurface,
        root_size: Option<Vec2>,
        node_transform_query: &mut Query<(
            &mut Node,
            &mut Transform,
            &Style,
            Option<&BorderRadius>,
            Option<&Outline>,
            Option<&ScrollPosition>,
        )>,
        ui_children: &UiChildren,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
        parent_scroll_position: Vec2,
        mut absolute_location: Vec2,
    ) {
        if let Ok((
            mut node,
            mut transform,
            style,
            maybe_border_radius,
            maybe_outline,
            maybe_scroll_position,
        )) = node_transform_query.get_mut(entity)
        {
            let Ok(layout) = ui_surface.get_layout(entity) else {
                return;
            };

            let layout_size =
                inverse_target_scale_factor * Vec2::new(layout.size.width, layout.size.height);
            let layout_location =
                inverse_target_scale_factor * Vec2::new(layout.location.x, layout.location.y);

            absolute_location += layout_location;

            let rounded_size = approx_round_layout_coords(absolute_location + layout_size)
                - approx_round_layout_coords(absolute_location);

            let rounded_location =
                approx_round_layout_coords(layout_location - parent_scroll_position)
                    + 0.5 * (rounded_size - parent_size);

            // only trigger change detection when the new values are different
            if node.calculated_size != rounded_size || node.unrounded_size != layout_size {
                node.calculated_size = rounded_size;
                node.unrounded_size = layout_size;
            }

            node.bypass_change_detection().border = BorderRect {
                left: layout.border.left * inverse_target_scale_factor,
                right: layout.border.right * inverse_target_scale_factor,
                top: layout.border.top * inverse_target_scale_factor,
                bottom: layout.border.bottom * inverse_target_scale_factor,
            };

            let viewport_size = root_size.unwrap_or(node.calculated_size);

            if let Some(border_radius) = maybe_border_radius {
                // We don't trigger change detection for changes to border radius
                node.bypass_change_detection().border_radius =
                    border_radius.resolve(node.calculated_size, viewport_size);
            }

            if let Some(outline) = maybe_outline {
                // don't trigger change detection when only outlines are changed
                let node = node.bypass_change_detection();
                node.outline_width = if style.display != Display::None {
                    outline
                        .width
                        .resolve(node.size().x, viewport_size)
                        .unwrap_or(0.)
                        .max(0.)
                } else {
                    0.
                };

                node.outline_offset = outline
                    .offset
                    .resolve(node.size().x, viewport_size)
                    .unwrap_or(0.)
                    .max(0.);
            }

            if transform.translation.truncate() != rounded_location {
                transform.translation = rounded_location.extend(0.);
            }

            let scroll_position: Vec2 = maybe_scroll_position
                .map(|scroll_pos| {
                    Vec2::new(
                        if style.overflow.x == OverflowAxis::Scroll {
                            scroll_pos.offset_x
                        } else {
                            0.0
                        },
                        if style.overflow.y == OverflowAxis::Scroll {
                            scroll_pos.offset_y
                        } else {
                            0.0
                        },
                    )
                })
                .unwrap_or_default();

            let round_content_size = approx_round_layout_coords(
                Vec2::new(layout.content_size.width, layout.content_size.height)
                    * inverse_target_scale_factor,
            );
            let max_possible_offset = (round_content_size - rounded_size).max(Vec2::ZERO);
            let clamped_scroll_position = scroll_position.clamp(Vec2::ZERO, max_possible_offset);

            if clamped_scroll_position != scroll_position {
                commands
                    .entity(entity)
                    .insert(ScrollPosition::from(&clamped_scroll_position));
            }

            for child_uinode in ui_children.iter_ui_children(entity) {
                update_uinode_geometry_recursive(
                    commands,
                    child_uinode,
                    ui_surface,
                    Some(viewport_size),
                    node_transform_query,
                    ui_children,
                    inverse_target_scale_factor,
                    rounded_size,
                    clamped_scroll_position,
                    absolute_location,
                );
            }
        }
    }
}

#[inline]
/// Round `value` to the nearest whole integer, with ties (values with a fractional part equal to 0.5) rounded towards positive infinity.
fn approx_round_ties_up(value: f32) -> f32 {
    (value + 0.5).floor()
}

#[inline]
/// Rounds layout coordinates by rounding ties upwards.
///
/// Rounding ties up avoids gaining a pixel when rounding bounds that span from negative to positive.
///
/// Example: The width between bounds of -50.5 and 49.5 before rounding is 100, using:
/// - `f32::round`: width becomes 101 (rounds to -51 and 50).
/// - `round_ties_up`: width is 100 (rounds to -50 and 50).
fn approx_round_layout_coords(value: Vec2) -> Vec2 {
    Vec2 {
        x: approx_round_ties_up(value.x),
        y: approx_round_ties_up(value.y),
    }
}

#[cfg(test)]
mod tests {
    use taffy::TraversePartialTree;

    use bevy_asset::{AssetEvent, Assets};
    use bevy_core_pipeline::core_2d::Camera2d;
    use bevy_ecs::{
        entity::Entity,
        event::Events,
        prelude::{Commands, Component, In, Query, With},
        query::Without,
        schedule::{apply_deferred, IntoSystemConfigs, Schedule},
        system::RunSystemOnce,
        world::World,
    };
    use bevy_hierarchy::{
        despawn_with_children_recursive, BuildChildren, ChildBuild, Children, Parent,
    };
    use bevy_math::{vec2, Rect, UVec2, Vec2};
    use bevy_render::{
        camera::{ManualTextureViews, OrthographicProjection},
        prelude::Camera,
        texture::Image,
    };
    use bevy_transform::{
        prelude::GlobalTransform,
        systems::{propagate_transforms, sync_simple_transforms},
    };
    use bevy_utils::{prelude::default, HashMap};
    use bevy_window::{
        PrimaryWindow, Window, WindowCreated, WindowResized, WindowResolution,
        WindowScaleFactorChanged,
    };

    use crate::{
        layout::{approx_round_layout_coords, ui_surface::UiSurface},
        prelude::*,
        ui_layout_system,
        update::update_target_camera_system,
        ContentSize,
    };

    #[test]
    fn round_layout_coords_must_round_ties_up() {
        assert_eq!(
            approx_round_layout_coords(vec2(-50.5, 49.5)),
            vec2(-50., 50.)
        );
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
        #[cfg(feature = "bevy_text")]
        world.init_resource::<bevy_text::TextPipeline>();
        #[cfg(feature = "bevy_text")]
        world.init_resource::<bevy_text::CosmicFontSystem>();
        #[cfg(feature = "bevy_text")]
        world.init_resource::<bevy_text::SwashCache>();

        // spawn a dummy primary window and camera
        world.spawn((
            Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                ..default()
            },
            PrimaryWindow,
        ));
        world.spawn(Camera2d);

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
        let camera_entity = world.spawn(Camera2d).id();

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
        despawn_with_children_recursive(&mut world, ui_parent_entity, true);

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
                    let current_rect = Rect::from_center_size(
                        global_transform.translation().truncate(),
                        node.size(),
                    );
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
            world.run_system_once_with(new_pos, move_ui_node).unwrap();
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

        world.spawn((
            Camera2d,
            Camera {
                order: 1,
                ..default()
            },
        ));

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

        world.run_system_once(update_camera_viewports).unwrap();

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
        #[cfg(feature = "bevy_text")]
        world.init_resource::<bevy_text::TextPipeline>();
        #[cfg(feature = "bevy_text")]
        world.init_resource::<bevy_text::CosmicFontSystem>();
        #[cfg(feature = "bevy_text")]
        world.init_resource::<bevy_text::SwashCache>();

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
