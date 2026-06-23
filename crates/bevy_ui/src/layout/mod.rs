use crate::{
    experimental::{UiChildren, UiRootNodes},
    ui_transform::{UiGlobalTransform, UiTransform},
    ComputedNode, ComputedUiRenderTargetInfo, ContentSize, Display, FixedNode, IgnoreScroll,
    LayoutConfig, Node, Outline, OverflowAxis, ScrollPosition,
};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::RemovedComponents,
    query::{Added, Has, With},
    system::{ParamSet, Query, ResMut},
    world::Ref,
};

use bevy_math::{Affine2, Vec2};
use bevy_sprite::BorderRect;
use thiserror::Error;
use ui_surface::{ComputedLayout, UiSurface};

use bevy_text::ComputedTextBlock;

use bevy_text::FontCx;

mod convert;
pub mod debug;
mod style;
pub mod ui_surface;

#[derive(Copy, Clone)]
pub struct LayoutContext {
    pub scale_factor: f32,
    pub physical_size: Vec2,
}

impl LayoutContext {
    pub const DEFAULT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::ZERO,
    };
    /// Create a new [`LayoutContext`] from the window's physical size and scale factor
    #[inline]
    const fn new(scale_factor: f32, physical_size: Vec2) -> Self {
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
    TaffyError(taffy::tree::TaffyError),
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
pub fn ui_layout_system(
    mut ui_surface: ResMut<UiSurface>,
    ui_root_node_query: UiRootNodes,
    fixed_nodes_query: Query<Entity, (With<FixedNode>, With<ChildOf>)>,
    ui_children: UiChildren,
    node_query: Query<(Ref<Node>, Ref<ComputedUiRenderTargetInfo>)>,
    content_size_query: Query<Ref<ContentSize>>,
    mut node_queries: ParamSet<(
        Query<&mut ComputedLayout>,
        Query<(
            &mut ComputedNode,
            &UiTransform,
            &mut UiGlobalTransform,
            &Node,
            &ComputedLayout,
            Option<&LayoutConfig>,
            Option<&Outline>,
            Option<&ScrollPosition>,
            Option<&IgnoreScroll>,
            Has<FixedNode>,
        )>,
        Query<(&mut ComputedNode, &mut UiGlobalTransform, &ComputedLayout)>,
    )>,
    mut buffer_query: Query<&mut ComputedTextBlock>,
    mut font_system: ResMut<FontCx>,
    added_fixed_node_query: Query<Entity, Added<FixedNode>>,
    mut removed_fixed_nodes: RemovedComponents<FixedNode>,
) {
    for mut computed_layout in &mut node_queries.p0() {
        computed_layout
            .bypass_change_detection()
            .prepare_for_layout();
    }

    let fixed_node_changes = added_fixed_node_query
        .iter()
        .chain(removed_fixed_nodes.read())
        .collect::<Vec<_>>();

    for ui_root_entity in ui_root_node_query.iter().chain(fixed_nodes_query.iter()) {
        let Ok((physical_size, scale_factor)) =
            node_query.get(ui_root_entity).map(|(_, computed_target)| {
                (
                    computed_target.physical_size(),
                    computed_target.scale_factor(),
                )
            })
        else {
            continue;
        };

        let computed = {
            let mut computed_layout_query = node_queries.p0();
            ui_surface.compute_layout(
                ui_root_entity,
                physical_size,
                &ui_children,
                &node_query,
                &content_size_query,
                &mut computed_layout_query,
                &fixed_nodes_query,
                &fixed_node_changes,
                &mut buffer_query,
                &mut font_system,
            )
        };

        if computed.is_err() {
            continue;
        }

        let mut node_update_query = node_queries.p1();
        update_uinode_geometry_recursive(
            ui_root_entity,
            ui_root_entity,
            true,
            physical_size.as_vec2(),
            Affine2::IDENTITY,
            &mut node_update_query,
            &ui_children,
            scale_factor.recip(),
            Vec2::ZERO,
            Vec2::ZERO,
        );
    }

    for mut computed_layout in &mut node_queries.p0() {
        computed_layout
            .bypass_change_detection()
            .clear_if_unreachable();
    }

    for (mut node, mut global_transform, computed_layout) in &mut node_queries.p2() {
        if computed_layout.get(true).is_some() {
            continue;
        }

        if *node != ComputedNode::DEFAULT {
            *node = ComputedNode::DEFAULT;
        }

        if *global_transform != UiGlobalTransform::default() {
            *global_transform = UiGlobalTransform::default();
        }
    }

    // Returns the combined bounding box of the node and any of its overflowing children.
    fn update_uinode_geometry_recursive(
        root: Entity,
        entity: Entity,
        inherited_use_rounding: bool,
        target_size: Vec2,
        mut inherited_transform: Affine2,
        node_update_query: &mut Query<(
            &mut ComputedNode,
            &UiTransform,
            &mut UiGlobalTransform,
            &Node,
            &ComputedLayout,
            Option<&LayoutConfig>,
            Option<&Outline>,
            Option<&ScrollPosition>,
            Option<&IgnoreScroll>,
            Has<FixedNode>,
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
            computed_layout,
            maybe_layout_config,
            maybe_outline,
            maybe_scroll_position,
            maybe_scroll_sticky,
            is_fixed_node,
        )) = node_update_query.get_mut(entity)
        {
            if is_fixed_node && root != entity {
                return;
            }

            let use_rounding = maybe_layout_config
                .map(|layout_config| layout_config.use_rounding)
                .unwrap_or(inherited_use_rounding);

            let Some((layout, unrounded_size)) = computed_layout.get(use_rounding) else {
                return;
            };

            let layout_size = Vec2::new(layout.size.width, layout.size.height);

            // Taffy layout position of the top-left corner of the node, relative to its parent.
            let layout_location = Vec2::new(layout.location.x, layout.location.y);

            // If IgnoreScroll is set, parent scroll position is ignored along the specified axes.
            let effective_parent_scroll = maybe_scroll_sticky
                .map(|scroll_sticky| parent_scroll_position * Vec2::from(!scroll_sticky.0))
                .unwrap_or(parent_scroll_position);

            // The position of the center of the node relative to its top-left corner.
            let local_center =
                layout_location - effective_parent_scroll + 0.5 * (layout_size - parent_size);

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
                min_inset: Vec2::new(rect.left, rect.top),
                max_inset: Vec2::new(rect.right, rect.bottom),
            };

            node.bypass_change_detection().border = taffy_rect_to_border_rect(layout.border);
            node.bypass_change_detection().padding = taffy_rect_to_border_rect(layout.padding);

            // Compute the node's new global transform
            let mut local_transform = transform.compute_affine(
                inverse_target_scale_factor.recip(),
                layout_size,
                target_size,
            );
            local_transform.translation += local_center;
            inherited_transform *= local_transform;

            if inherited_transform != **global_transform {
                *global_transform = inherited_transform.into();
            }

            // We don't trigger change detection for changes to border radius
            node.bypass_change_detection().border_radius = style.border_radius.resolve(
                inverse_target_scale_factor.recip(),
                node.size,
                target_size,
            );

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
                    // Clamp outline offsets to at least the length of the node's shorter side
                    // Negative offset outlines can be useful to create thing like in-set focus indicators
                    .max(-0.5 * node.size.min_element());
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
                    root,
                    child_uinode,
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
    use crate::{
        layout::ui_surface::{ComputedLayout, UiSurface},
        prelude::*,
        ui_layout_system,
        update::propagate_ui_target_cameras,
        ContentSize,
    };
    use bevy_app::{App, HierarchyPropagatePlugin, PostUpdate, PropagateSet, TaskPoolPlugin};
    use bevy_camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
    use bevy_ecs::prelude::*;
    use bevy_math::{UVec2, Vec2};
    use bevy_transform::systems::mark_dirty_trees;
    use bevy_transform::systems::{propagate_parent_transforms, sync_simple_transforms};
    use bevy_utils::prelude::default;

    const TARGET_WIDTH: u32 = 1000;
    const TARGET_HEIGHT: u32 = 100;

    fn setup_ui_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default());

        app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
            PostUpdate,
        ));
        app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
            PostUpdate,
        ));
        app.init_resource::<UiScale>();
        app.init_resource::<UiSurface>();
        app.init_resource::<bevy_text::TextPipeline>();
        app.init_resource::<bevy_text::FontCx>();
        app.init_resource::<bevy_text::ScaleCx>();
        app.init_resource::<bevy_transform::StaticTransformOptimizations>();

        app.add_systems(
            PostUpdate,
            (
                ApplyDeferred,
                propagate_ui_target_cameras,
                ui_layout_system,
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiTargetCamera>::default()
                .after(propagate_ui_target_cameras)
                .before(ui_layout_system),
        );

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiRenderTargetInfo>::default()
                .after(propagate_ui_target_cameras)
                .before(ui_layout_system),
        );

        app.world_mut().spawn((
            Camera2d,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                        scale_factor: 1.,
                    }),
                    ..Default::default()
                },
                viewport: Some(Viewport {
                    physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                    ..default()
                }),
                ..Default::default()
            },
        ));

        app
    }

    fn layout_for(app: &App, entity: Entity, use_rounding: bool) -> taffy::Layout {
        app.world()
            .get::<ComputedLayout>(entity)
            .and_then(|layout| layout.get(use_rounding))
            .unwrap()
            .0
    }

    fn has_layout(app: &App, entity: Entity) -> bool {
        app.world()
            .get::<ComputedLayout>(entity)
            .and_then(|layout| layout.get(true))
            .is_some()
    }

    #[test]
    fn ui_nodes_with_percent_100_dimensions_should_fill_their_parent() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();
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

        for ui_entity in [ui_root, ui_child] {
            let layout = layout_for(&app, ui_entity, true);
            assert_eq!(layout.size.width, TARGET_WIDTH as f32);
            assert_eq!(layout.size.height, TARGET_HEIGHT as f32);
        }
    }

    #[test]
    fn computed_layout_lifecycle() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();

        let ui_entity = world.spawn(Node::default()).id();
        assert!(app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get(true))
            .is_none());

        app.update();
        assert!(app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get(true))
            .is_some());

        app.world_mut().despawn(ui_entity);
        app.update();
        assert!(app.world().get::<ComputedLayout>(ui_entity).is_none());
    }

    #[test]
    fn layouts_are_removed_when_nodes_despawn() {
        let mut app = setup_ui_test_app();
        let entity = app.world_mut().spawn(Node::default()).id();

        app.update();
        assert!(has_layout(&app, entity));

        app.world_mut().despawn(entity);
        app.update();

        assert!(!has_layout(&app, entity));
    }

    #[test]
    fn reparenting_recomputes_from_current_entity_tree() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let root_a = world
            .spawn(Node {
                width: px(100.),
                height: px(20.),
                ..default()
            })
            .id();
        let root_b = world
            .spawn(Node {
                width: px(200.),
                height: px(20.),
                ..default()
            })
            .id();
        let child = world
            .spawn(Node {
                width: percent(100.),
                height: px(10.),
                ..default()
            })
            .id();

        world.entity_mut(root_a).add_child(child);
        app.update();
        assert_eq!(layout_for(&app, child, true).size.width, 100.);

        let world = app.world_mut();
        world.entity_mut(root_a).detach_child(child);
        world.entity_mut(root_b).add_child(child);
        app.update();
        assert_eq!(layout_for(&app, child, true).size.width, 200.);
    }

    #[test]
    fn child_style_change_invalidates_parent_cache() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let child = world
            .spawn(Node {
                width: px(50.),
                height: px(10.),
                ..default()
            })
            .id();
        let root = world
            .spawn(Node {
                width: px(100.),
                height: px(20.),
                ..default()
            })
            .add_child(child)
            .id();

        app.update();
        app.update();

        app.world_mut()
            .entity_mut(child)
            .get_mut::<Node>()
            .unwrap()
            .width = px(75.);

        app.update();

        assert_eq!(layout_for(&app, root, true).size.width, 100.);
        assert_eq!(layout_for(&app, child, true).size.width, 75.);
    }

    #[test]
    fn ui_node_should_be_set_to_its_content_size() {
        let mut app = setup_ui_test_app();
        let content_size = Vec2::new(50., 25.);

        let ui_entity = app
            .world_mut()
            .spawn((
                Node {
                    align_self: AlignSelf::Start,
                    ..default()
                },
                ContentSize::fixed_size(content_size),
            ))
            .id();

        app.update();
        let layout = layout_for(&app, ui_entity, true);

        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);
    }

    #[test]
    fn measured_node_includes_border_and_padding() {
        let mut app = setup_ui_test_app();

        let ui_node = app
            .world_mut()
            .spawn((
                Node {
                    align_self: AlignSelf::Start,
                    border: UiRect {
                        left: px(2.0),
                        right: px(6.0),
                        top: px(4.0),
                        bottom: px(8.0),
                    },
                    padding: UiRect {
                        left: px(3.0),
                        right: px(5.0),
                        top: px(7.0),
                        bottom: px(11.0),
                    },
                    ..default()
                },
                ContentSize::fixed_size(Vec2::new(50.0, 25.0)),
            ))
            .id();

        app.update();
        let layout = layout_for(&app, ui_node, true);

        assert_eq!(layout.border.left, 2.0);
        assert_eq!(layout.border.right, 6.0);
        assert_eq!(layout.border.top, 4.0);
        assert_eq!(layout.border.bottom, 8.0);
        assert_eq!(layout.padding.left, 3.0);
        assert_eq!(layout.padding.right, 5.0);
        assert_eq!(layout.padding.top, 7.0);
        assert_eq!(layout.padding.bottom, 11.0);
        assert_eq!(layout.size.width, 66.0);
        assert_eq!(layout.size.height, 55.0);
        assert_eq!(layout.content_size.width, 58.0);
        assert_eq!(layout.content_size.height, 43.0);
        assert_eq!(layout.content_box_width(), 50.0);
        assert_eq!(layout.content_box_height(), 25.0);
    }

    #[test]
    fn measure_funcs_persist_until_cleared() {
        let mut app = setup_ui_test_app();
        let content_size = Vec2::new(50., 25.);
        let ui_entity = app
            .world_mut()
            .spawn((Node::default(), ContentSize::fixed_size(content_size)))
            .id();

        app.update();
        let layout = layout_for(&app, ui_entity, true);
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);

        app.update();
        let layout = layout_for(&app, ui_entity, true);
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);

        app.world_mut()
            .entity_mut(ui_entity)
            .get_mut::<ContentSize>()
            .unwrap()
            .clear();

        app.update();
        let layout = layout_for(&app, ui_entity, true);
        assert_eq!(layout.size.width, 0.);
        assert_eq!(layout.size.height, 0.);
    }

    #[test]
    fn get_layout_can_return_unrounded_layout() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let root = world
            .spawn(Node {
                width: px(101.),
                height: px(10.),
                ..default()
            })
            .id();
        let child = world
            .spawn(Node {
                width: percent(50.),
                height: px(10.),
                ..default()
            })
            .id();
        world.entity_mut(root).add_child(child);

        app.update();

        let rounded = layout_for(&app, child, true);
        let unrounded = layout_for(&app, child, false);
        assert_eq!(unrounded.size.width, 50.5);
        assert_ne!(rounded.size.width, unrounded.size.width);
    }

    #[test]
    fn fixed_child_uses_viewport_layout_context() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let parent = world
            .spawn(Node {
                width: px(200.),
                height: px(20.),
                ..default()
            })
            .id();
        let fixed = world
            .spawn((
                Node {
                    width: percent(50.),
                    height: px(10.),
                    ..default()
                },
                FixedNode,
                ChildOf(parent),
            ))
            .id();

        app.update();

        assert_eq!(layout_for(&app, parent, true).size.width, 200.);
        assert_eq!(
            layout_for(&app, fixed, true).size.width,
            TARGET_WIDTH as f32 * 0.5
        );
    }

    #[test]
    fn fixed_node_changes_recompute_parent_and_child_layouts() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let parent = world
            .spawn(Node {
                width: px(200.),
                height: px(20.),
                ..default()
            })
            .id();
        let child = world
            .spawn((
                Node {
                    width: percent(50.),
                    height: px(10.),
                    ..default()
                },
                FixedNode,
                ChildOf(parent),
            ))
            .id();

        app.update();
        assert_eq!(
            layout_for(&app, child, true).size.width,
            TARGET_WIDTH as f32 * 0.5
        );

        app.world_mut().entity_mut(child).remove::<FixedNode>();
        app.update();
        assert_eq!(layout_for(&app, child, true).size.width, 100.);

        app.world_mut().entity_mut(child).insert(FixedNode);
        app.update();
        assert_eq!(
            layout_for(&app, child, true).size.width,
            TARGET_WIDTH as f32 * 0.5
        );
    }

    #[test]
    fn ui_rounding_test() {
        let mut app = setup_ui_test_app();
        let parent = app
            .world_mut()
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

        let children = app
            .world()
            .entity(parent)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect::<Vec<Entity>>();

        for r in [2, 3, 5, 7, 11, 13, 17, 19, 21, 23, 29, 31].map(|n| (n as f32).recip()) {
            let mut s = 1. - r;
            while s <= 5. {
                app.world_mut().resource_mut::<UiScale>().0 = s;
                app.update();
                let world = app.world();
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
            (propagate_ui_target_cameras, ApplyDeferred, ui_layout_system).chain(),
        );

        app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
            PostUpdate,
        ));

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiTargetCamera>::default()
                .after(propagate_ui_target_cameras)
                .before(ui_layout_system),
        );

        let world = app.world_mut();
        world.init_resource::<UiScale>();
        world.init_resource::<UiSurface>();
        world.init_resource::<bevy_text::TextPipeline>();
        world.init_resource::<bevy_text::FontCx>();
        world.init_resource::<bevy_text::ScaleCx>();

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

    #[cfg(feature = "ghost_nodes")]
    mod ghost_node_tests {
        use super::*;
        use crate::experimental::GhostNode;

        #[test]
        fn ghost_nodes_flatten_layout_children() {
            let mut app = setup_ui_test_app();
            let world = app.world_mut();

            let child = world
                .spawn(Node {
                    width: px(50.),
                    height: px(10.),
                    ..default()
                })
                .id();
            let mid = world.spawn(GhostNode).add_child(child).id();
            let root = world
                .spawn(Node {
                    width: px(100.),
                    height: px(20.),
                    ..default()
                })
                .add_child(mid)
                .id();

            app.update();
            assert!(has_layout(&app, child));

            app.world_mut().entity_mut(mid).remove::<GhostNode>();
            app.update();
            assert!(!has_layout(&app, child));
            assert_eq!(
                app.world().get::<ComputedNode>(child).unwrap().size(),
                Vec2::ZERO
            );

            app.world_mut().entity_mut(mid).insert(GhostNode);
            app.update();
            let root_layout = layout_for(&app, root, true);
            let child_layout = layout_for(&app, child, true);
            assert_eq!(root_layout.size.width, 100.);
            assert_eq!(child_layout.size.width, 50.);
        }
    }
}
