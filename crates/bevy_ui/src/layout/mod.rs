use crate::{
    experimental::{UiChildren, UiRootNodes},
    ui_transform::{UiGlobalTransform, UiTransform},
    BorderRadius, ComputedNode, ComputedNodeTarget, ContentSize, Display, LayoutConfig, Node,
    Outline, OverflowAxis, ScrollPosition,
};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::Added,
    system::{Query, ResMut},
    world::Ref,
};

use bevy_math::{Affine2, Vec2};
use bevy_sprite::BorderRect;
use thiserror::Error;
use ui_surface::UiSurface;

use bevy_text::ComputedTextBlock;

use bevy_text::CosmicFontSystem;

mod convert;
pub mod debug;
pub(crate) mod ui_surface;

pub struct LayoutContext {
    pub scale_factor: f32,
    pub physical_size: Vec2,
}

impl LayoutContext {
    pub const DEFAULT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::ZERO,
    };
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    fn new(scale_factor: f32, physical_size: Vec2) -> Self {
        Self {
            scale_factor,
            physical_size,
        }
    }
}

#[cfg(test)]
impl LayoutContext {
    pub const TEST_CONTEXT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::new(1000.0, 1000.0),
    };
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
    TaffyError(taffy::TaffyError),
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
pub fn ui_layout_system(
    mut ui_surface: ResMut<UiSurface>,
    ui_root_node_query: UiRootNodes,
    ui_children: UiChildren,
    mut node_query: Query<(
        Entity,
        Ref<Node>,
        Option<&mut ContentSize>,
        Ref<ComputedNodeTarget>,
    )>,
    added_node_query: Query<(), Added<Node>>,
    mut node_update_query: Query<(
        &mut ComputedNode,
        &UiTransform,
        &mut UiGlobalTransform,
        &Node,
        Option<&LayoutConfig>,
        Option<&BorderRadius>,
        Option<&Outline>,
        Option<&ScrollPosition>,
    )>,
    mut buffer_query: Query<&mut ComputedTextBlock>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut removed_nodes: RemovedComponents<Node>,
) {
    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_content_sizes.read() {
        ui_surface.try_remove_node_context(entity);
    }

    // Sync Node and ContentSize to Taffy for all nodes
    node_query
        .iter_mut()
        .for_each(|(entity, node, content_size, computed_target)| {
            if computed_target.is_changed()
                || node.is_changed()
                || content_size
                    .as_ref()
                    .is_some_and(|c| c.is_changed() || c.measure.is_some())
            {
                let layout_context = LayoutContext::new(
                    computed_target.scale_factor,
                    computed_target.physical_size.as_vec2(),
                );
                let measure = content_size.and_then(|mut c| c.measure.take());
                ui_surface.upsert_node(&layout_context, entity, &node, measure);
            }
        });

    // update and remove children
    for entity in removed_children.read() {
        ui_surface.try_remove_children(entity);
    }

    // clean up removed nodes after syncing children to avoid potential panic (invalid SlotMap key used)
    ui_surface.remove_entities(
        removed_nodes
            .read()
            .filter(|entity| !node_query.contains(*entity)),
    );

    for ui_root_entity in ui_root_node_query.iter() {
        fn update_children_recursively(
            ui_surface: &mut UiSurface,
            ui_children: &UiChildren,
            added_node_query: &Query<(), Added<Node>>,
            entity: Entity,
        ) {
            if ui_surface.entity_to_taffy.contains_key(&entity)
                && (added_node_query.contains(entity)
                    || ui_children.is_changed(entity)
                    || ui_children
                        .iter_ui_children(entity)
                        .any(|child| added_node_query.contains(child)))
            {
                ui_surface.update_children(entity, ui_children.iter_ui_children(entity));
            }

            for child in ui_children.iter_ui_children(entity) {
                update_children_recursively(ui_surface, ui_children, added_node_query, child);
            }
        }

        update_children_recursively(
            &mut ui_surface,
            &ui_children,
            &added_node_query,
            ui_root_entity,
        );

        let (_, _, _, computed_target) = node_query.get(ui_root_entity).unwrap();

        ui_surface.compute_layout(
            ui_root_entity,
            computed_target.physical_size,
            &mut buffer_query,
            &mut font_system,
        );

        update_uinode_geometry_recursive(
            ui_root_entity,
            &mut ui_surface,
            true,
            computed_target.physical_size().as_vec2(),
            Affine2::IDENTITY,
            &mut node_update_query,
            &ui_children,
            computed_target.scale_factor.recip(),
            Vec2::ZERO,
            Vec2::ZERO,
        );
    }

    // Returns the combined bounding box of the node and any of its overflowing children.
    fn update_uinode_geometry_recursive(
        entity: Entity,
        ui_surface: &mut UiSurface,
        inherited_use_rounding: bool,
        target_size: Vec2,
        mut inherited_transform: Affine2,
        node_update_query: &mut Query<(
            &mut ComputedNode,
            &UiTransform,
            &mut UiGlobalTransform,
            &Node,
            Option<&LayoutConfig>,
            Option<&BorderRadius>,
            Option<&Outline>,
            Option<&ScrollPosition>,
        )>,
        ui_children: &UiChildren,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
        parent_scroll_position: Vec2,
    ) {
        if let Ok((
            mut node,
            transform,
            mut global_transform,
            style,
            maybe_layout_config,
            maybe_border_radius,
            maybe_outline,
            maybe_scroll_position,
        )) = node_update_query.get_mut(entity)
        {
            let use_rounding = maybe_layout_config
                .map(|layout_config| layout_config.use_rounding)
                .unwrap_or(inherited_use_rounding);

            let Ok((layout, unrounded_size)) = ui_surface.get_layout(entity, use_rounding) else {
                return;
            };

            let layout_size = Vec2::new(layout.size.width, layout.size.height);

            // Taffy layout position of the top-left corner of the node, relative to its parent.
            let layout_location = Vec2::new(layout.location.x, layout.location.y);

            // The position of the center of the node relative to its top-left corner.
            let local_center =
                layout_location - parent_scroll_position + 0.5 * (layout_size - parent_size);

            // only trigger change detection when the new values are different
            if node.size != layout_size
                || node.unrounded_size != unrounded_size
                || node.inverse_scale_factor != inverse_target_scale_factor
            {
                node.size = layout_size;
                node.unrounded_size = unrounded_size;
                node.inverse_scale_factor = inverse_target_scale_factor;
            }

            let content_size = Vec2::new(layout.content_size.width, layout.content_size.height);
            node.bypass_change_detection().content_size = content_size;

            let taffy_rect_to_border_rect = |rect: taffy::Rect<f32>| BorderRect {
                left: rect.left,
                right: rect.right,
                top: rect.top,
                bottom: rect.bottom,
            };

            node.bypass_change_detection().border = taffy_rect_to_border_rect(layout.border);
            node.bypass_change_detection().padding = taffy_rect_to_border_rect(layout.padding);

            // Computer the node's new global transform
            let mut local_transform =
                transform.compute_affine(inverse_target_scale_factor, layout_size, target_size);
            local_transform.translation += local_center;
            inherited_transform *= local_transform;

            if inherited_transform != **global_transform {
                *global_transform = inherited_transform.into();
            }

            if let Some(border_radius) = maybe_border_radius {
                // We don't trigger change detection for changes to border radius
                node.bypass_change_detection().border_radius = border_radius.resolve(
                    inverse_target_scale_factor.recip(),
                    node.size,
                    target_size,
                );
            }

            if let Some(outline) = maybe_outline {
                // don't trigger change detection when only outlines are changed
                let node = node.bypass_change_detection();
                node.outline_width = if style.display != Display::None {
                    outline
                        .width
                        .resolve(
                            inverse_target_scale_factor.recip(),
                            node.size().x,
                            target_size,
                        )
                        .unwrap_or(0.)
                        .max(0.)
                } else {
                    0.
                };

                node.outline_offset = outline
                    .offset
                    .resolve(
                        inverse_target_scale_factor.recip(),
                        node.size().x,
                        target_size,
                    )
                    .unwrap_or(0.)
                    .max(0.);
            }

            node.bypass_change_detection().scrollbar_size =
                Vec2::new(layout.scrollbar_size.width, layout.scrollbar_size.height);

            let scroll_position: Vec2 = maybe_scroll_position
                .map(|scroll_pos| {
                    Vec2::new(
                        if style.overflow.x == OverflowAxis::Scroll {
                            scroll_pos.x * inverse_target_scale_factor.recip()
                        } else {
                            0.0
                        },
                        if style.overflow.y == OverflowAxis::Scroll {
                            scroll_pos.y * inverse_target_scale_factor.recip()
                        } else {
                            0.0
                        },
                    )
                })
                .unwrap_or_default();

            let max_possible_offset =
                (content_size - layout_size + node.scrollbar_size).max(Vec2::ZERO);
            let clamped_scroll_position = scroll_position.clamp(Vec2::ZERO, max_possible_offset);

            let physical_scroll_position = clamped_scroll_position.floor();

            node.bypass_change_detection().scroll_position = physical_scroll_position;

            for child_uinode in ui_children.iter_ui_children(entity) {
                update_uinode_geometry_recursive(
                    child_uinode,
                    ui_surface,
                    use_rounding,
                    target_size,
                    inherited_transform,
                    node_update_query,
                    ui_children,
                    inverse_target_scale_factor,
                    layout_size,
                    physical_scroll_position,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, HierarchyPropagatePlugin, PostUpdate, PropagateSet};
    use taffy::TraversePartialTree;

    use bevy_asset::{AssetEvent, Assets};
    use bevy_camera::{Camera, Camera2d};
    use bevy_ecs::{prelude::*, system::RunSystemOnce};
    use bevy_image::Image;
    use bevy_math::{Rect, UVec2, Vec2};
    use bevy_platform::collections::HashMap;
    use bevy_render::texture::ManualTextureViews;
    use bevy_transform::systems::mark_dirty_trees;
    use bevy_transform::systems::{propagate_parent_transforms, sync_simple_transforms};
    use bevy_utils::prelude::default;
    use bevy_window::{
        PrimaryWindow, Window, WindowCreated, WindowResized, WindowResolution,
        WindowScaleFactorChanged,
    };
    //use uuid::timestamp::UUID_TICKS_BETWEEN_EPOCHS;

    use crate::{
        layout::ui_surface::UiSurface, prelude::*, ui_layout_system,
        update::update_ui_context_system, ContentSize, LayoutContext,
    };

    // these window dimensions are easy to convert to and from percentage values
    const WINDOW_WIDTH: f32 = 1000.;
    const WINDOW_HEIGHT: f32 = 100.;

    fn setup_ui_test_app() -> App {
        let mut app = App::new();

        app.add_plugins(HierarchyPropagatePlugin::<ComputedNodeTarget>::new(
            PostUpdate,
        ));
        app.init_resource::<UiScale>();
        app.init_resource::<UiSurface>();
        app.init_resource::<Events<WindowScaleFactorChanged>>();
        app.init_resource::<Events<WindowResized>>();
        // Required for the camera system
        app.init_resource::<Events<WindowCreated>>();
        app.init_resource::<Events<AssetEvent<Image>>>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<ManualTextureViews>();
        app.init_resource::<bevy_text::TextPipeline>();
        app.init_resource::<bevy_text::CosmicFontSystem>();
        app.init_resource::<bevy_text::SwashCache>();

        app.add_systems(
            PostUpdate,
            (
                // UI is driven by calculated camera target info, so we need to run the camera system first
                bevy_render::camera::camera_system,
                update_ui_context_system,
                ApplyDeferred,
                ui_layout_system,
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedNodeTarget>::default()
                .after(update_ui_context_system)
                .before(ui_layout_system),
        );

        let world = app.world_mut();
        // spawn a dummy primary window and camera
        world.spawn((
            Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                ..default()
            },
            PrimaryWindow,
        ));
        world.spawn(Camera2d);

        app
    }

    #[test]
    fn ui_nodes_with_percent_100_dimensions_should_fill_their_parent() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();

        // spawn a root entity with width and height set to fill 100% of its parent
        let ui_root = world
            .spawn(Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            })
            .id();

        let ui_child = world
            .spawn(Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            })
            .id();

        world.entity_mut(ui_root).add_child(ui_child);

        app.update();

        let mut ui_surface = app.world_mut().resource_mut::<UiSurface>();

        for ui_entity in [ui_root, ui_child] {
            let layout = ui_surface.get_layout(ui_entity, true).unwrap().0;
            assert_eq!(layout.size.width, WINDOW_WIDTH);
            assert_eq!(layout.size.height, WINDOW_HEIGHT);
        }
    }

    #[test]
    fn ui_surface_tracks_ui_entities() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();
        // no UI entities in world, none in UiSurface
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.is_empty());

        let ui_entity = world.spawn(Node::default()).id();

        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert_eq!(ui_surface.entity_to_taffy.len(), 1);

        world.despawn(ui_entity);

        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();
        assert!(!ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert!(ui_surface.entity_to_taffy.is_empty());
    }

    #[test]
    #[should_panic]
    fn despawning_a_ui_entity_should_remove_its_corresponding_ui_node() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let ui_entity = world.spawn(Node::default()).id();

        // `ui_layout_system` will insert a ui node into the internal layout tree corresponding to `ui_entity`
        app.update();
        let world = app.world_mut();

        // retrieve the ui node corresponding to `ui_entity` from ui surface
        let ui_surface = world.resource::<UiSurface>();
        let ui_node = ui_surface.entity_to_taffy[&ui_entity];

        world.despawn(ui_entity);

        // `ui_layout_system` will receive a `RemovedComponents<Node>` event for `ui_entity`
        // and remove `ui_entity` from `ui_node` from the internal layout tree
        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();

        // `ui_node` is removed, attempting to retrieve a style for `ui_node` panics
        let _ = ui_surface.taffy.style(ui_node.id);
    }

    #[test]
    fn changes_to_children_of_a_ui_entity_change_its_corresponding_ui_nodes_children() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let ui_parent_entity = world.spawn(Node::default()).id();

        // `ui_layout_system` will insert a ui node into the internal layout tree corresponding to `ui_entity`
        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();
        let ui_parent_node = ui_surface.entity_to_taffy[&ui_parent_entity];

        // `ui_parent_node` shouldn't have any children yet
        assert_eq!(ui_surface.taffy.child_count(ui_parent_node.id), 0);

        let mut ui_child_entities = (0..10)
            .map(|_| {
                let child = world.spawn(Node::default()).id();
                world.entity_mut(ui_parent_entity).add_child(child);
                child
            })
            .collect::<Vec<_>>();

        app.update();
        let world = app.world_mut();

        // `ui_parent_node` should have children now
        let ui_surface = world.resource::<UiSurface>();
        assert_eq!(
            ui_surface.entity_to_taffy.len(),
            1 + ui_child_entities.len()
        );
        assert_eq!(
            ui_surface.taffy.child_count(ui_parent_node.id),
            ui_child_entities.len()
        );

        let child_node_map = <HashMap<_, _>>::from_iter(
            ui_child_entities
                .iter()
                .map(|child_entity| (*child_entity, ui_surface.entity_to_taffy[child_entity])),
        );

        // the children should have a corresponding ui node and that ui node's parent should be `ui_parent_node`
        for node in child_node_map.values() {
            assert_eq!(ui_surface.taffy.parent(node.id), Some(ui_parent_node.id));
        }

        // delete every second child
        let mut deleted_children = vec![];
        for i in (0..ui_child_entities.len()).rev().step_by(2) {
            let child = ui_child_entities.remove(i);
            world.despawn(child);
            deleted_children.push(child);
        }

        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();
        assert_eq!(
            ui_surface.entity_to_taffy.len(),
            1 + ui_child_entities.len()
        );
        assert_eq!(
            ui_surface.taffy.child_count(ui_parent_node.id),
            ui_child_entities.len()
        );

        // the remaining children should still have nodes in the layout tree
        for child_entity in &ui_child_entities {
            let child_node = child_node_map[child_entity];
            assert_eq!(ui_surface.entity_to_taffy[child_entity], child_node);
            assert_eq!(
                ui_surface.taffy.parent(child_node.id),
                Some(ui_parent_node.id)
            );
            assert!(ui_surface
                .taffy
                .children(ui_parent_node.id)
                .unwrap()
                .contains(&child_node.id));
        }

        // the nodes of the deleted children should have been removed from the layout tree
        for deleted_child_entity in &deleted_children {
            assert!(!ui_surface
                .entity_to_taffy
                .contains_key(deleted_child_entity));
            let deleted_child_node = child_node_map[deleted_child_entity];
            assert!(!ui_surface
                .taffy
                .children(ui_parent_node.id)
                .unwrap()
                .contains(&deleted_child_node.id));
        }

        // despawn the parent entity and its descendants
        world.entity_mut(ui_parent_entity).despawn();

        app.update();
        let world = app.world_mut();

        // all nodes should have been deleted
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.is_empty());
    }

    /// bugfix test, see [#16288](https://github.com/bevyengine/bevy/pull/16288)
    #[test]
    fn node_removal_and_reinsert_should_work() {
        let mut app = setup_ui_test_app();

        app.update();
        let world = app.world_mut();

        // no UI entities in world, none in UiSurface
        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.is_empty());

        let ui_entity = world.spawn(Node::default()).id();

        // `ui_layout_system` should map `ui_entity` to a ui node in `UiSurface::entity_to_taffy`
        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert_eq!(ui_surface.entity_to_taffy.len(), 1);

        // remove and re-insert Node to trigger removal code in `ui_layout_system`
        world.entity_mut(ui_entity).remove::<Node>();
        world.entity_mut(ui_entity).insert(Node::default());

        // `ui_layout_system` should still have `ui_entity`
        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();
        assert!(ui_surface.entity_to_taffy.contains_key(&ui_entity));
        assert_eq!(ui_surface.entity_to_taffy.len(), 1);
    }

    #[test]
    fn node_addition_should_sync_children() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        // spawn an invalid UI root node
        let root_node = world.spawn(()).with_child(Node::default()).id();

        app.update();
        let world = app.world_mut();

        // fix the invalid root node by inserting a Node
        world.entity_mut(root_node).insert(Node::default());

        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource_mut::<UiSurface>();
        let taffy_root = ui_surface.entity_to_taffy[&root_node];

        // There should be one child of the root node after fixing it
        assert_eq!(ui_surface.taffy.child_count(taffy_root.id), 1);
    }

    #[test]
    fn node_addition_should_sync_parent_and_children() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let d = world.spawn(Node::default()).id();
        let c = world.spawn(()).add_child(d).id();
        let b = world.spawn(Node::default()).id();
        let a = world.spawn(Node::default()).add_children(&[b, c]).id();

        app.update();
        let world = app.world_mut();

        // fix the invalid middle node by inserting a Node
        world.entity_mut(c).insert(Node::default());

        app.update();
        let world = app.world_mut();

        let ui_surface = world.resource::<UiSurface>();
        for (entity, n) in [(a, 2), (b, 0), (c, 1), (d, 0)] {
            let taffy_id = ui_surface.entity_to_taffy[&entity].id;
            assert_eq!(ui_surface.taffy.child_count(taffy_id), n);
        }
    }

    /// regression test for >=0.13.1 root node layouts
    /// ensure root nodes act like they are absolutely positioned
    /// without explicitly declaring it.
    #[test]
    fn ui_root_node_should_act_like_position_absolute() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let mut size = 150.;

        world.spawn(Node {
            // test should pass without explicitly requiring position_type to be set to Absolute
            // position_type: PositionType::Absolute,
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        });

        size -= 50.;

        world.spawn(Node {
            // position_type: PositionType::Absolute,
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        });

        size -= 50.;

        world.spawn(Node {
            // position_type: PositionType::Absolute,
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        });

        app.update();
        let world = app.world_mut();

        let overlap_check = world
            .query_filtered::<(Entity, &ComputedNode, &UiGlobalTransform), Without<ChildOf>>()
            .iter(world)
            .fold(
                Option::<(Rect, bool)>::None,
                |option_rect, (entity, node, transform)| {
                    let current_rect = Rect::from_center_size(transform.translation, node.size());
                    assert!(
                        current_rect.height().abs() + current_rect.width().abs() > 0.,
                        "root ui node {entity} doesn't have a logical size"
                    );
                    assert_ne!(
                        *transform,
                        UiGlobalTransform::default(),
                        "root ui node {entity} transform is not populated"
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
                .single()
                .expect("missing primary window");
            let camera_count = cameras.iter().len();
            for (camera_index, mut camera) in cameras.iter_mut().enumerate() {
                let viewport_width =
                    primary_window.resolution.physical_width() / camera_count as u32;
                let viewport_height = primary_window.resolution.physical_height();
                let physical_position = UVec2::new(viewport_width * camera_index as u32, 0);
                let physical_size = UVec2::new(viewport_width, viewport_height);
                camera.viewport = Some(bevy_camera::Viewport {
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
                    .insert(UiTargetCamera(target_camera_entity))
                    .insert(Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(pos.y),
                        left: Val::Px(pos.x),
                        ..default()
                    });
            }
        }

        fn do_move_and_test(app: &mut App, new_pos: Vec2, expected_camera_entity: &Entity) {
            let world = app.world_mut();
            world.run_system_once_with(move_ui_node, new_pos).unwrap();
            app.update();
            let world = app.world_mut();
            let (ui_node_entity, UiTargetCamera(target_camera_entity)) = world
                .query_filtered::<(Entity, &UiTargetCamera), With<MovingUiNode>>()
                .single(world)
                .expect("missing MovingUiNode");
            assert_eq!(expected_camera_entity, target_camera_entity);
            let mut ui_surface = world.resource_mut::<UiSurface>();

            let layout = ui_surface
                .get_layout(ui_node_entity, true)
                .expect("failed to get layout")
                .0;

            // negative test for #12255
            assert_eq!(Vec2::new(layout.location.x, layout.location.y), new_pos);
        }

        fn get_taffy_node_count(world: &World) -> usize {
            world.resource::<UiSurface>().taffy.total_node_count()
        }

        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        world.spawn((
            Camera2d,
            Camera {
                order: 1,
                ..default()
            },
        ));

        world.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                left: Val::Px(0.),
                ..default()
            },
            MovingUiNode,
        ));

        app.update();
        let world = app.world_mut();

        let pos_inc = Vec2::splat(1.);
        let total_cameras = world.query::<&Camera>().iter(world).len();
        // add total cameras - 1 (the assumed default) to get an idea for how many nodes we should expect
        let expected_max_taffy_node_count = get_taffy_node_count(world) + total_cameras - 1;

        world.run_system_once(update_camera_viewports).unwrap();

        app.update();
        let world = app.world_mut();

        let viewport_rects = world
            .query::<(Entity, &Camera)>()
            .iter(world)
            .map(|(e, c)| (e, c.logical_viewport_rect().expect("missing viewport")))
            .collect::<Vec<_>>();

        for (camera_entity, viewport) in viewport_rects.iter() {
            let target_pos = viewport.min + pos_inc;
            do_move_and_test(&mut app, target_pos, camera_entity);
        }

        // reverse direction
        let mut viewport_rects = viewport_rects.clone();
        viewport_rects.reverse();
        for (camera_entity, viewport) in viewport_rects.iter() {
            let target_pos = viewport.max - pos_inc;
            do_move_and_test(&mut app, target_pos, camera_entity);
        }

        let world = app.world();
        let current_taffy_node_count = get_taffy_node_count(world);
        if current_taffy_node_count > expected_max_taffy_node_count {
            panic!("extra taffy nodes detected: current: {current_taffy_node_count} max expected: {expected_max_taffy_node_count}");
        }
    }

    #[test]
    fn ui_node_should_be_set_to_its_content_size() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let content_size = Vec2::new(50., 25.);

        let ui_entity = world
            .spawn((
                Node {
                    align_self: AlignSelf::Start,
                    ..default()
                },
                ContentSize::fixed_size(content_size),
            ))
            .id();

        app.update();
        let world = app.world_mut();

        let mut ui_surface = world.resource_mut::<UiSurface>();
        let layout = ui_surface.get_layout(ui_entity, true).unwrap().0;

        // the node should takes its size from the fixed size measure func
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);
    }

    #[test]
    fn measure_funcs_should_be_removed_on_content_size_removal() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let content_size = Vec2::new(50., 25.);
        let ui_entity = world
            .spawn((
                Node {
                    align_self: AlignSelf::Start,
                    ..Default::default()
                },
                ContentSize::fixed_size(content_size),
            ))
            .id();

        app.update();
        let world = app.world_mut();

        let mut ui_surface = world.resource_mut::<UiSurface>();
        let ui_node = ui_surface.entity_to_taffy[&ui_entity];

        // a node with a content size should have taffy context
        assert!(ui_surface.taffy.get_node_context(ui_node.id).is_some());
        let layout = ui_surface.get_layout(ui_entity, true).unwrap().0;
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);

        world.entity_mut(ui_entity).remove::<ContentSize>();

        app.update();
        let world = app.world_mut();

        let mut ui_surface = world.resource_mut::<UiSurface>();
        // a node without a content size should not have taffy context
        assert!(ui_surface.taffy.get_node_context(ui_node.id).is_none());

        // Without a content size, the node has no width or height constraints so the length of both dimensions is 0.
        let layout = ui_surface.get_layout(ui_entity, true).unwrap().0;
        assert_eq!(layout.size.width, 0.);
        assert_eq!(layout.size.height, 0.);
    }

    #[test]
    fn ui_rounding_test() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let parent = world
            .spawn(Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::min_content(2),
                margin: UiRect::all(Val::Px(4.0)),
                ..default()
            })
            .with_children(|commands| {
                for _ in 0..2 {
                    commands.spawn(Node {
                        display: Display::Grid,
                        width: Val::Px(160.),
                        height: Val::Px(160.),
                        ..default()
                    });
                }
            })
            .id();

        let children = world
            .entity(parent)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect::<Vec<Entity>>();

        for r in [2, 3, 5, 7, 11, 13, 17, 19, 21, 23, 29, 31].map(|n| (n as f32).recip()) {
            // This fails with very small / unrealistic scale values
            let mut s = 1. - r;
            while s <= 5. {
                app.world_mut().resource_mut::<UiScale>().0 = s;
                app.update();
                let world = app.world_mut();
                let width_sum: f32 = children
                    .iter()
                    .map(|child| world.get::<ComputedNode>(*child).unwrap().size.x)
                    .sum();
                let parent_width = world.get::<ComputedNode>(parent).unwrap().size.x;
                assert!((width_sum - parent_width).abs() < 0.001);
                assert!((width_sum - 320. * s).abs() <= 1.);
                s += r;
            }
        }
    }

    #[test]
    fn no_camera_ui() {
        let mut app = App::new();

        app.add_systems(
            PostUpdate,
            (
                // UI is driven by calculated camera target info, so we need to run the camera system first
                bevy_render::camera::camera_system,
                update_ui_context_system,
                ApplyDeferred,
                ui_layout_system,
            )
                .chain(),
        );

        app.add_plugins(HierarchyPropagatePlugin::<ComputedNodeTarget>::new(
            PostUpdate,
        ));

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedNodeTarget>::default()
                .after(update_ui_context_system)
                .before(ui_layout_system),
        );

        let world = app.world_mut();
        world.init_resource::<UiScale>();
        world.init_resource::<UiSurface>();
        world.init_resource::<Events<WindowScaleFactorChanged>>();
        world.init_resource::<Events<WindowResized>>();
        // Required for the camera system
        world.init_resource::<Events<WindowCreated>>();
        world.init_resource::<Events<AssetEvent<Image>>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<ManualTextureViews>();

        world.init_resource::<bevy_text::TextPipeline>();

        world.init_resource::<bevy_text::CosmicFontSystem>();

        world.init_resource::<bevy_text::SwashCache>();

        // spawn a dummy primary window and camera
        world.spawn((
            Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                ..default()
            },
            PrimaryWindow,
        ));

        let ui_root = world
            .spawn(Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            })
            .id();

        let ui_child = world
            .spawn(Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            })
            .id();

        world.entity_mut(ui_root).add_child(ui_child);

        app.update();
    }

    #[test]
    fn test_ui_surface_compute_camera_layout() {
        use bevy_ecs::prelude::ResMut;

        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let root_node_entity = Entity::from_raw_u32(1).unwrap();

        struct TestSystemParam {
            root_node_entity: Entity,
        }

        fn test_system(
            params: In<TestSystemParam>,
            mut ui_surface: ResMut<UiSurface>,
            mut computed_text_block_query: Query<&mut bevy_text::ComputedTextBlock>,
            mut font_system: ResMut<bevy_text::CosmicFontSystem>,
        ) {
            ui_surface.upsert_node(
                &LayoutContext::TEST_CONTEXT,
                params.root_node_entity,
                &Node::default(),
                None,
            );

            ui_surface.compute_layout(
                params.root_node_entity,
                UVec2::new(800, 600),
                &mut computed_text_block_query,
                &mut font_system,
            );
        }

        let _ = world.run_system_once_with(test_system, TestSystemParam { root_node_entity });

        let ui_surface = world.resource::<UiSurface>();

        let taffy_node = ui_surface.entity_to_taffy.get(&root_node_entity).unwrap();
        assert!(ui_surface.taffy.layout(taffy_node.id).is_ok());
    }

    #[test]
    fn no_viewport_node_leak_on_root_despawned() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let ui_root_entity = world.spawn(Node::default()).id();

        // The UI schedule synchronizes Bevy UI's internal `TaffyTree` with the
        // main world's tree of `Node` entities.
        app.update();
        let world = app.world_mut();

        // Two taffy nodes are added to the internal `TaffyTree` for each root UI entity.
        // An implicit taffy node representing the viewport and a taffy node corresponding to the
        // root UI entity which is parented to the viewport taffy node.
        assert_eq!(
            world.resource_mut::<UiSurface>().taffy.total_node_count(),
            2
        );

        world.despawn(ui_root_entity);

        // The UI schedule removes both the taffy node corresponding to `ui_root_entity` and its
        // parent viewport node.
        app.update();
        let world = app.world_mut();

        // Both taffy nodes should now be removed from the internal `TaffyTree`
        assert_eq!(
            world.resource_mut::<UiSurface>().taffy.total_node_count(),
            0
        );
    }

    #[test]
    fn no_viewport_node_leak_on_parented_root() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let ui_root_entity_1 = world.spawn(Node::default()).id();
        let ui_root_entity_2 = world.spawn(Node::default()).id();

        app.update();
        let world = app.world_mut();

        // There are two UI root entities. Each root taffy node is given it's own viewport node parent,
        // so a total of four taffy nodes are added to the `TaffyTree` by the UI schedule.
        assert_eq!(
            world.resource_mut::<UiSurface>().taffy.total_node_count(),
            4
        );

        // Parent `ui_root_entity_2` onto `ui_root_entity_1` so now only `ui_root_entity_1` is a
        // UI root entity.
        world
            .entity_mut(ui_root_entity_1)
            .add_child(ui_root_entity_2);

        // Now there is only one root node so the second viewport node is removed by
        // the UI schedule.
        app.update();
        let world = app.world_mut();

        // There is only one viewport node now, so the `TaffyTree` contains 3 nodes in total.
        assert_eq!(
            world.resource_mut::<UiSurface>().taffy.total_node_count(),
            3
        );
    }
}
