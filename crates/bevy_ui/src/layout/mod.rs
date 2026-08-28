use crate::{
    experimental::{UiChildren, UiRootNodes},
    layout_tree::{compute_layout, node_id_entity, TaffyStyle},
    ui_transform::{UiGlobalTransform, UiTransform},
    ComputedNode, ComputedUiRenderTargetInfo, ContentSize, Display, FixedNode, IgnoreScroll,
    LayoutConfig, Node, Outline, OverflowAxis, ScrollPosition,
};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    system::{Local, ParamSet, Query, Res, ResMut},
    world::Ref,
};

use bevy_math::{Affine2, Vec2};
use bevy_sprite::BorderRect;
use layout_tree::ComputedLayout;
use thiserror::Error;

use bevy_text::{ComputedTextBlock, EmSize, FontCx, RemSize, TextFont, DEFAULT_REM_SIZE_PX};

mod convert;
pub mod debug;
pub mod layout_tree;

#[derive(Copy, Clone)]
pub struct LayoutContext {
    pub scale_factor: f32,
    pub physical_size: Vec2,
    pub em_size: f32,
    pub rem_size: f32,
}

impl LayoutContext {
    pub const DEFAULT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::ZERO,
        em_size: DEFAULT_REM_SIZE_PX,
        rem_size: DEFAULT_REM_SIZE_PX,
    };
    /// Create a new [`LayoutContext`] from the window's physical size and scale factor
    #[inline]
    const fn new(
        scale_factor: f32,
        physical_size: Vec2,
        em_size: EmSize,
        rem_size: RemSize,
    ) -> Self {
        Self {
            scale_factor,
            physical_size,
            em_size: em_size.0,
            rem_size: rem_size.0,
        }
    }
}

#[cfg(test)]
impl LayoutContext {
    pub const TEST_CONTEXT: Self = Self {
        physical_size: Vec2::new(1000.0, 1000.0),
        ..Self::DEFAULT
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

/// For any entity with a [`TextFont`], set [`EmSize`] to the font size resolved
/// into pixels when the `TextFont`, render target or `RemSize` changes. Nodes
/// without `TextFont` keep their `EmSize` intact. If `TextFont` is removed the
/// `EmSize` remains unchanged.
pub fn sync_font_size_to_em_size(
    mut em_size_query: Query<
        (&mut EmSize, Ref<TextFont>, Ref<ComputedUiRenderTargetInfo>),
        With<Node>,
    >,
    rem_size: Res<RemSize>,
) {
    // `Val::Rem` resolves from rem size so need to recalc when this changes
    let rem_size_changed = rem_size.is_changed();

    for (mut em_size, text_font, computed_ui_render_target_info) in em_size_query.iter_mut() {
        if text_font.is_changed() || computed_ui_render_target_info.is_changed() || rem_size_changed
        {
            em_size.set_if_neq(EmSize::from_font_size(
                text_font.font_size,
                computed_ui_render_target_info.logical_size(),
                *rem_size,
            ));
        }
    }
}

pub fn update_taffy_styles(
    rem_size: Res<RemSize>,
    mut update_query: Query<(
        Ref<Node>,
        Ref<ComputedUiRenderTargetInfo>,
        Ref<EmSize>,
        &mut TaffyStyle,
    )>,
) {
    update_query
        .par_iter_mut()
        .for_each(|(node, target, em_size, mut taffy_style)| {
            if node.is_changed()
                || target.is_changed()
                || em_size.is_changed()
                || rem_size.is_changed()
            {
                convert::update_taffy_style_from_node(
                    &node,
                    &LayoutContext::new(
                        target.scale_factor(),
                        target.physical_size().as_vec2(),
                        *em_size,
                        *rem_size,
                    ),
                    &mut taffy_style,
                );
            }
        });
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
pub fn ui_layout_system(
    ui_root_node_query: UiRootNodes,
    fixed_nodes_query: Query<Entity, (With<FixedNode>, With<ChildOf>)>,
    ui_children: UiChildren,
    target_query: Query<Ref<ComputedUiRenderTargetInfo>>,
    node_query: Query<
        (
            Ref<TaffyStyle>,
            Ref<ContentSize>,
            Has<FixedNode>,
            Ref<UiTransform>,
            Ref<ScrollPosition>,
            Option<Ref<Outline>>,
            Option<Ref<LayoutConfig>>,
            Option<Ref<IgnoreScroll>>,
        ),
        With<Node>,
    >,
    style_query: Query<&TaffyStyle>,
    mut node_queries: ParamSet<(
        Query<&mut ComputedLayout>,
        Query<(
            &mut ComputedNode,
            &mut UiGlobalTransform,
            &mut ComputedLayout,
        )>,
    )>,
    mut buffer_query: Query<&mut ComputedTextBlock>,
    mut font_system: ResMut<FontCx>,
    added_fixed_node_query: Query<Entity, Added<FixedNode>>,
    mut removed_fixed_nodes: RemovedComponents<FixedNode>,
    rem_size: Res<RemSize>,
    mut child_stack: Local<Vec<taffy::NodeId>>,
) {
    let fixed_node_changes = added_fixed_node_query
        .iter()
        .chain(removed_fixed_nodes.read())
        .collect::<Vec<_>>();

    let mut computed_layout_query = node_queries.p0();
    for ui_root_entity in ui_root_node_query.iter().chain(fixed_nodes_query.iter()) {
        let Ok(target) = target_query.get(ui_root_entity) else {
            continue;
        };

        let _ = compute_layout(
            ui_root_entity,
            target.physical_size(),
            &ui_children,
            &node_query,
            &style_query,
            &mut computed_layout_query,
            &fixed_node_changes,
            &mut buffer_query,
            &mut font_system,
            *rem_size,
            &mut child_stack,
        );
        child_stack.clear();
    }

    node_queries.p1().par_iter_mut().for_each(
        |(mut node, mut global_transform, mut computed_layout)| {
            if !computed_layout.visited() {
                computed_layout.clear();
            }
            computed_layout.set_visited(false);

            if computed_layout.has_layout() {
                return;
            }

            if *node != ComputedNode::DEFAULT {
                *node = ComputedNode::DEFAULT;
            }

            if *global_transform != UiGlobalTransform::default() {
                *global_transform = UiGlobalTransform::default();
            }
        },
    );
}

pub fn update_computed_nodes(
    rem_size: Res<RemSize>,
    ui_root_node_query: UiRootNodes,
    fixed_nodes_query: Query<Entity, (With<FixedNode>, With<ChildOf>)>,
    targets_query: Query<Ref<ComputedUiRenderTargetInfo>>,
    mut computed_nodes_query: Query<(
        &mut ComputedNode,
        &UiTransform,
        &mut UiGlobalTransform,
        &Node,
        &ComputedLayout,
        &EmSize,
        Option<&LayoutConfig>,
        Option<&Outline>,
        Option<&ScrollPosition>,
        Option<&IgnoreScroll>,
    )>,
    mut child_stack: Local<Vec<taffy::NodeId>>,
) {
    for ui_root_entity in ui_root_node_query.iter().chain(fixed_nodes_query.iter()) {
        let Ok(target_info) = targets_query.get(ui_root_entity) else {
            continue;
        };
        update_uinode_geometry_recursive(
            ui_root_entity,
            ui_root_entity,
            true,
            target_info.physical_size().as_vec2(),
            Affine2::IDENTITY,
            &mut computed_nodes_query,
            target_info.scale_factor().recip(),
            Vec2::ZERO,
            Vec2::ZERO,
            *rem_size,
            &mut child_stack,
            target_info.is_changed() | rem_size.is_changed(),
        );
        child_stack.clear();
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
        &EmSize,
        Option<&LayoutConfig>,
        Option<&Outline>,
        Option<&ScrollPosition>,
        Option<&IgnoreScroll>,
    )>,
    inverse_target_scale_factor: f32,
    parent_size: Vec2,
    parent_scroll_position: Vec2,
    rem_size: RemSize,
    child_stack: &mut Vec<taffy::NodeId>,
    force_update: bool,
) {
    if let Ok((
        mut node,
        transform,
        mut global_transform,
        style,
        computed_layout,
        em_size,
        maybe_layout_config,
        maybe_outline,
        maybe_scroll_position,
        maybe_scroll_sticky,
    )) = node_update_query.get_mut(entity)
    {
        if !force_update && !computed_layout.layout_changed() && !computed_layout.subtree_dirty() {
            return;
        }

        let use_rounding = maybe_layout_config
            .map(|layout_config| layout_config.use_rounding)
            .unwrap_or(inherited_use_rounding);

        let Some((layout, unrounded_size)) = computed_layout.get_layout(use_rounding) else {
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

        let content_size = Vec2::new(
            layout.scrollable_overflow_rect.right,
            layout.scrollable_overflow_rect.bottom,
        );
        if node.content_size != content_size {
            node.content_size = content_size;
        }

        let taffy_rect_to_border_rect = |rect: taffy::Rect<f32>| BorderRect {
            min_inset: Vec2::new(rect.left, rect.top),
            max_inset: Vec2::new(rect.right, rect.bottom),
        };

        let new_border = taffy_rect_to_border_rect(layout.border);
        if node.border != new_border {
            node.border = new_border;
        }
        let new_padding = taffy_rect_to_border_rect(layout.padding);
        if node.padding != new_padding {
            node.padding = new_padding;
        }

        if node.em_size != *em_size {
            node.em_size = *em_size;
        }
        if node.rem_size != rem_size {
            node.rem_size = rem_size;
        }

        // Compute the node's new global transform
        let mut local_transform = transform.compute_affine(
            inverse_target_scale_factor.recip(),
            layout_size,
            target_size,
            *em_size,
            rem_size,
        );
        local_transform.translation += local_center;
        inherited_transform *= local_transform;

        if inherited_transform != **global_transform {
            *global_transform = inherited_transform.into();
        }

        if let Some(outline) = maybe_outline {
            // don't trigger change detection unless the outline actually changed
            let new_outline_width = if style.display != Display::None {
                outline
                    .width
                    .resolve(
                        inverse_target_scale_factor.recip(),
                        node.size().x,
                        target_size,
                        *em_size,
                        rem_size,
                    )
                    .unwrap_or(0.)
                    .max(0.)
            } else {
                0.
            };

            if node.outline_width != new_outline_width {
                node.outline_width = new_outline_width;
            }

            let new_outline_offset = outline
                .offset
                .resolve(
                    inverse_target_scale_factor.recip(),
                    node.size().x,
                    target_size,
                    *em_size,
                    rem_size,
                )
                .unwrap_or(0.)
                // Clamp outline offsets to at least the length of the node's shorter side
                // Negative offset outlines can be useful to create thing like in-set focus indicators
                .max(-0.5 * node.size.min_element());
            if node.outline_offset != new_outline_offset {
                node.outline_offset = new_outline_offset;
            }
        } else if node.outline_width != 0. || node.outline_offset != 0. {
            node.outline_width = 0.;
            node.outline_offset = 0.;
        }

        let new_scrollbar_size =
            Vec2::new(layout.scrollbar_size.width, layout.scrollbar_size.height);
        if node.scrollbar_size != new_scrollbar_size {
            node.scrollbar_size = new_scrollbar_size;
        }

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

        if node.scroll_position != physical_scroll_position {
            node.scroll_position = physical_scroll_position;
        }

        let start = child_stack.len();
        child_stack.extend_from_slice(computed_layout.child_nodes());
        let end = child_stack.len();

        let inherited_force_update =
            force_update || computed_layout.layout_changed() || computed_layout.self_dirty();
        for child_index in start..end {
            update_uinode_geometry_recursive(
                root,
                node_id_entity(child_stack[child_index]),
                use_rounding,
                target_size,
                inherited_transform,
                node_update_query,
                inverse_target_scale_factor,
                layout_size,
                physical_scroll_position,
                rem_size,
                child_stack,
                inherited_force_update,
            );
        }

        child_stack.truncate(start);
    }
}

pub fn update_border_radius(
    mut node_update_query: Query<
        (&mut ComputedNode, &Node, &ComputedUiRenderTargetInfo),
        Or<(
            Changed<ComputedNode>,
            Changed<Node>,
            Changed<ComputedUiRenderTargetInfo>,
        )>,
    >,
) {
    node_update_query
        .par_iter_mut()
        .for_each(|(mut node, style, target)| {
            // We don't trigger change detection for changes to border radius
            // unless the border radius actually changed
            let new_border_radius = style.border_radius.resolve(
                node.inverse_scale_factor.recip(),
                node.size,
                target.physical_size.as_vec2(),
                node.em_size,
                node.rem_size,
            );
            if node.border_radius != new_border_radius {
                node.border_radius = new_border_radius;
            }
        });
}

#[cfg(test)]
mod tests {
    use crate::layout_tree::compute_layout;
    use crate::layout_tree::TaffyStyle;
    use crate::update_border_radius;
    use crate::update_computed_nodes;
    use crate::UiSystems;
    use crate::{
        experimental::UiChildren, layout::layout_tree::ComputedLayout, prelude::*,
        sync_font_size_to_em_size, ui_layout_system, update::propagate_ui_target_cameras,
        update_taffy_styles, ContentSize,
    };
    use bevy_app::{App, HierarchyPropagatePlugin, PostUpdate, PropagateSet, TaskPoolPlugin};
    use bevy_camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
    use bevy_ecs::{prelude::*, system::RunSystemOnce, world::Ref};
    use bevy_math::{BVec2, Rect, UVec2, Vec2};
    use bevy_text::TextFont;
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
        app.init_resource::<bevy_text::TextPipeline>();
        app.init_resource::<bevy_text::FontCx>();
        app.init_resource::<RemSize>();
        app.init_resource::<bevy_text::ScaleCx>();
        app.init_resource::<bevy_transform::StaticTransformOptimizations>();

        app.add_systems(
            PostUpdate,
            (
                ApplyDeferred,
                propagate_ui_target_cameras,
                sync_font_size_to_em_size,
                update_taffy_styles,
                ui_layout_system,
                update_computed_nodes,
                update_border_radius,
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
                .before(update_taffy_styles),
        );

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiRenderTargetInfo>::default()
                .after(propagate_ui_target_cameras)
                .before(update_taffy_styles),
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
            let layout = app
                .world()
                .get::<ComputedLayout>(ui_entity)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0;
            assert_eq!(layout.size.width, TARGET_WIDTH as f32);
            assert_eq!(layout.size.height, TARGET_HEIGHT as f32);
        }
    }

    #[test]
    fn computed_layout_lifecycle() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();

        let ui_entity = world.spawn(Node::default()).id();
        assert!(!app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .is_some_and(ComputedLayout::has_layout));

        app.update();
        assert!(app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .is_some_and(ComputedLayout::has_layout));

        app.world_mut().despawn(ui_entity);
        app.update();
        assert!(app.world().get::<ComputedLayout>(ui_entity).is_none());
    }

    #[test]
    fn layouts_are_removed_when_nodes_despawn() {
        let mut app = setup_ui_test_app();
        let entity = app.world_mut().spawn(Node::default()).id();

        app.update();
        assert!(app
            .world()
            .get::<ComputedLayout>(entity)
            .is_some_and(ComputedLayout::has_layout));

        app.world_mut().despawn(entity);
        app.update();

        assert!(!app
            .world()
            .get::<ComputedLayout>(entity)
            .is_some_and(ComputedLayout::has_layout));
    }

    #[test]
    fn node_removal_and_reinsert_should_work() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, (With<Node>, With<ComputedLayout>)>()
                .iter(world)
                .count(),
            0
        );

        let ui_entity = world.spawn(Node::default()).id();

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, (With<Node>, With<ComputedLayout>)>()
                .single(world)
                .unwrap(),
            ui_entity
        );

        app.world_mut().entity_mut(ui_entity).remove::<Node>();
        app.world_mut().entity_mut(ui_entity).insert(Node {
            width: px(100.),
            ..default()
        });

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, (With<Node>, With<ComputedLayout>)>()
                .single(world)
                .unwrap(),
            ui_entity
        );
        assert_eq!(
            world
                .get::<ComputedLayout>(ui_entity)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            100.
        );
    }

    #[test]
    fn node_addition_should_sync_children() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        // spawn an invalid UI root node
        let child = world.spawn(Node::default()).id();
        let root = world.spawn(()).add_child(child).id();

        app.update();
        assert!(!app
            .world()
            .get::<ComputedLayout>(child)
            .is_some_and(ComputedLayout::has_layout));

        // fix the invalid root node by inserting a Node
        app.world_mut().entity_mut(root).insert(Node::default());

        app.update();
        // The root node's child should have a layout after update
        assert!(app
            .world()
            .get::<ComputedLayout>(child)
            .is_some_and(ComputedLayout::has_layout));
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
        assert!(!app
            .world()
            .get::<ComputedLayout>(d)
            .is_some_and(ComputedLayout::has_layout));

        // fix the invalid middle node by inserting a Node
        app.world_mut().entity_mut(c).insert(Node::default());

        app.update();
        for entity in [a, b, c, d] {
            assert!(app
                .world()
                .get::<ComputedLayout>(entity)
                .is_some_and(ComputedLayout::has_layout));
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

        fn update_camera_viewports(mut cameras: Query<&mut Camera>) {
            let camera_count = cameras.iter().len();
            for (camera_index, mut camera) in cameras.iter_mut().enumerate() {
                let target_size = camera.physical_target_size().unwrap();
                let viewport_width = target_size.x / camera_count as u32;
                let physical_position = UVec2::new(viewport_width * camera_index as u32, 0);
                let physical_size = UVec2::new(target_size.x / camera_count as u32, target_size.y);
                camera.viewport = Some(Viewport {
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

            let layout = world
                .get::<ComputedLayout>(ui_node_entity)
                .and_then(|layout| layout.get_layout(true))
                .expect("failed to get layout")
                .0;

            // negative test for #12255
            assert_eq!(Vec2::new(layout.location.x, layout.location.y), new_pos);
        }

        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        world.spawn((
            Camera2d,
            Camera {
                order: 1,
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                        scale_factor: 1.,
                    }),
                    ..default()
                },
                viewport: Some(Viewport {
                    physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                    ..default()
                }),
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
    }

    #[test]
    fn compute_layout_uses_camera_viewport() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let root_node_entity = world.spawn(Node::default()).id();

        fn test_system(
            In(root_node_entity): In<Entity>,
            ui_children: UiChildren,
            node_query: Query<
                (
                    Ref<TaffyStyle>,
                    Ref<ContentSize>,
                    Has<FixedNode>,
                    Ref<UiTransform>,
                    Ref<ScrollPosition>,
                    Option<Ref<Outline>>,
                    Option<Ref<LayoutConfig>>,
                    Option<Ref<IgnoreScroll>>,
                ),
                With<Node>,
            >,
            style_query: Query<&TaffyStyle>,
            mut node_queries: ParamSet<(Query<&mut ComputedLayout>,)>,
            mut buffer_query: Query<&mut bevy_text::ComputedTextBlock>,
            mut font_system: ResMut<bevy_text::FontCx>,
            rem_size: Res<RemSize>,
            mut child_stack: Local<Vec<taffy::NodeId>>,
        ) {
            compute_layout(
                root_node_entity,
                UVec2::new(800, 600),
                &ui_children,
                &node_query,
                &style_query,
                &mut node_queries.p0(),
                &[],
                &mut buffer_query,
                &mut font_system,
                *rem_size,
                &mut child_stack,
            )
            .unwrap();
        }

        world
            .run_system_once_with(test_system, root_node_entity)
            .unwrap();

        assert!(world
            .get::<ComputedLayout>(root_node_entity)
            .is_some_and(ComputedLayout::has_layout));
    }

    #[test]
    fn fixed_root_is_a_root_node() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();
        let fixed_entity = world
            .spawn((
                Node {
                    width: Val::Percent(50.),
                    height: Val::Percent(50.),
                    ..default()
                },
                FixedNode,
            ))
            .id();

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(fixed_entity)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5
        );
    }

    #[test]
    fn swap_fixed_nodes() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let a = world
            .spawn(Node {
                width: Val::Percent(20.),
                height: Val::Percent(20.),
                ..default()
            })
            .id();
        let b = world
            .spawn((
                Node {
                    width: Val::Percent(50.),
                    height: Val::Percent(50.),
                    ..default()
                },
                ChildOf(a),
            ))
            .id();

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(a)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.2
        );
        assert_eq!(
            world
                .get::<ComputedLayout>(b)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.2 * 0.5
        );

        world.entity_mut(a).insert(FixedNode);

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(b)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.2 * 0.5
        );

        world.entity_mut(b).insert(FixedNode);

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(b)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5
        );

        world.entity_mut(b).remove::<ChildOf>().add_child(a);

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(a)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.2
        );
        assert_eq!(
            world
                .get::<ComputedLayout>(b)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5
        );

        world.entity_mut(b).remove::<FixedNode>();

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(a)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.2
        );

        world.entity_mut(a).remove::<FixedNode>();

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(a)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5 * 0.2
        );
    }

    #[test]
    fn fixed_node_children() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();

        let a = world
            .spawn(Node {
                width: Val::Percent(50.),
                height: Val::Percent(50.),
                ..default()
            })
            .id();
        let b = world
            .spawn(Node {
                width: Val::Percent(50.),
                height: Val::Percent(50.),
                ..default()
            })
            .id();
        let c = world
            .spawn(Node {
                width: Val::Percent(50.),
                height: Val::Percent(50.),
                ..default()
            })
            .id();
        let p = world
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                width: Val::Px(200.),
                height: Val::Px(100.),
                ..default()
            })
            .add_children(&[a, b, c])
            .id();

        app.update();
        let world = app.world_mut();
        for entity in [a, b, c] {
            assert_eq!(
                world
                    .get::<ComputedLayout>(entity)
                    .and_then(|layout| layout.get_layout(true))
                    .unwrap()
                    .0
                    .size
                    .width,
                100.
            );
        }

        world.entity_mut(a).insert(FixedNode);

        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(a)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5
        );
        for entity in [b, c] {
            assert_eq!(
                world
                    .get::<ComputedLayout>(entity)
                    .and_then(|layout| layout.get_layout(true))
                    .unwrap()
                    .0
                    .size
                    .width,
                100.
            );
        }

        world.entity_mut(c).insert(FixedNode);
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .get::<ComputedLayout>(b)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            100.
        );
        for entity in [a, c] {
            assert_eq!(
                world
                    .get::<ComputedLayout>(entity)
                    .and_then(|layout| layout.get_layout(true))
                    .unwrap()
                    .0
                    .size
                    .width,
                TARGET_WIDTH as f32 * 0.5
            );
        }

        world.entity_mut(p).detach_all_children();
        world.entity_mut(p).despawn();

        app.update();
        let world = app.world_mut();
        for entity in [a, b, c] {
            assert_eq!(
                world
                    .get::<ComputedLayout>(entity)
                    .and_then(|layout| layout.get_layout(true))
                    .unwrap()
                    .0
                    .size
                    .width,
                TARGET_WIDTH as f32 * 0.5
            );
        }
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
        assert_eq!(
            app.world()
                .get::<ComputedLayout>(child)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            100.
        );

        let world = app.world_mut();
        world.entity_mut(root_a).detach_child(child);
        world.entity_mut(root_b).add_child(child);
        app.update();
        assert_eq!(
            app.world()
                .get::<ComputedLayout>(child)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            200.
        );
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

        assert_eq!(
            app.world()
                .get::<ComputedLayout>(root)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            100.
        );
        assert_eq!(
            app.world()
                .get::<ComputedLayout>(child)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            75.
        );
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
        let layout = app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0;

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
        let layout = app
            .world()
            .get::<ComputedLayout>(ui_node)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0;

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
        assert_eq!(
            app.world()
                .get::<ComputedNode>(ui_node)
                .unwrap()
                .padding_box()
                .size(),
            Vec2::new(58.0, 43.0)
        );
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
        let layout = app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0;
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);

        app.update();
        let layout = app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0;
        assert_eq!(layout.size.width, content_size.x);
        assert_eq!(layout.size.height, content_size.y);

        app.world_mut()
            .entity_mut(ui_entity)
            .get_mut::<ContentSize>()
            .unwrap()
            .clear();

        app.update();
        let layout = app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0;
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

        let rounded = app
            .world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0;
        let unrounded = app
            .world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(false))
            .unwrap()
            .0;
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

        assert_eq!(
            app.world()
                .get::<ComputedLayout>(parent)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            200.
        );
        assert_eq!(
            app.world()
                .get::<ComputedLayout>(fixed)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
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
            app.world()
                .get::<ComputedLayout>(child)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5
        );

        app.world_mut().entity_mut(child).remove::<FixedNode>();
        app.update();
        assert_eq!(
            app.world()
                .get::<ComputedLayout>(child)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            100.
        );

        app.world_mut().entity_mut(child).insert(FixedNode);
        app.update();
        assert_eq!(
            app.world()
                .get::<ComputedLayout>(child)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
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
            (propagate_ui_target_cameras, ApplyDeferred)
                .chain()
                .before(UiSystems::Layout),
        );

        app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
            PostUpdate,
        ));

        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiTargetCamera>::default()
                .after(propagate_ui_target_cameras)
                .before(UiSystems::Layout),
        );

        let world = app.world_mut();
        world.init_resource::<UiScale>();
        world.init_resource::<bevy_text::TextPipeline>();
        world.init_resource::<bevy_text::FontCx>();
        world.init_resource::<RemSize>();
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

    #[test]
    fn rem_sized_node_is_rem_sized() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();

        let ui_root = world
            .spawn(Node {
                width: Val::Rem(3.),
                height: Val::Rem(2.),
                ..default()
            })
            .id();

        app.update();

        let world = app.world_mut();

        let rem_size = world.resource::<RemSize>();

        let c = world.entity(ui_root).get::<ComputedNode>().unwrap();

        assert!(c.size().abs_diff_eq(rem_size.0 * Vec2::new(3., 2.), 1e-5));
    }

    #[test]
    fn em_and_rem_sized_nodes_are_updated_on_changes_to_em_and_rem_sizes() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();

        let ui_root = world
            .spawn((
                Node {
                    width: Val::Rem(20.),
                    height: Val::Em(30.),
                    ..default()
                },
                TextFont::default().with_font_size(5.),
            ))
            .id();

        let child = world
            .spawn((
                Node {
                    width: Val::Em(5.),
                    height: Val::Rem(4.),
                    ..default()
                },
                TextFont::default().with_font_size(15.),
                ChildOf(ui_root),
            ))
            .id();

        app.update();

        let world = app.world_mut();

        world.resource_mut::<RemSize>().0 = 10.;

        app.update();
        let world = app.world_mut();

        let computed_root = world.entity(ui_root).get::<ComputedNode>().unwrap();

        assert!(computed_root
            .size()
            .abs_diff_eq(Vec2::new(200., 150.), 1e-5));
        let computed_child = world.entity(child).get::<ComputedNode>().unwrap();
        assert!(computed_child.size().abs_diff_eq(Vec2::new(75., 40.), 1e-5));
    }

    #[test]
    fn removing_node_from_ui_child_should_relayout_parent() {
        let mut app = setup_ui_test_app();

        let world = app.world_mut();
        let ui_root = world.spawn(Node::default()).id();
        let ui_child = world
            .spawn((
                Node {
                    width: px(50.),
                    height: px(30.),
                    ..default()
                },
                ChildOf(ui_root),
            ))
            .id();

        app.update();

        let world = app.world_mut();
        world.entity_mut(ui_child).remove::<Node>();

        app.update();

        let world = app.world_mut();
        assert!(world
            .entity(ui_root)
            .get::<ComputedNode>()
            .unwrap()
            .size()
            .abs_diff_eq(Vec2::ZERO, 1e-5));
    }

    #[test]
    fn block_layouts_margins_collapse() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();
        let a = world
            .spawn(Node {
                height: px(50),
                margin: px(100).bottom(),
                ..default()
            })
            .id();
        let b = world
            .spawn(Node {
                height: px(50),
                margin: px(50).top(),
                ..default()
            })
            .id();
        world
            .spawn(Node {
                display: Display::Block,
                ..default()
            })
            .add_children(&[a, b]);

        app.update();

        let world = app.world();
        let computed_a = world.get::<ComputedNode>(a).unwrap();
        let transform_a = world.get::<UiGlobalTransform>(a).unwrap();
        let computed_b = world.get::<ComputedNode>(b).unwrap();
        let transform_b = world.get::<UiGlobalTransform>(b).unwrap();
        let a_bottom = 0.5 * computed_a.size.y + transform_a.affine().translation.y;
        let b_top = -0.5 * computed_b.size.y + transform_b.affine().translation.y;
        assert!((b_top - a_bottom - 100.).abs() <= 1e-5);
    }

    #[test]
    fn block_layouts_nested_margins_collapse() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();
        let a = world
            .spawn(Node {
                height: px(50),
                ..default()
            })
            .id();
        let nested_child = world
            .spawn(Node {
                display: Display::Block,
                margin: UiRect::vertical(px(40)),
                ..default()
            })
            .id();
        let nested = world
            .spawn(Node {
                display: Display::Block,
                ..default()
            })
            .add_child(nested_child)
            .id();
        let b = world
            .spawn(Node {
                height: px(50),
                ..default()
            })
            .id();
        world
            .spawn(Node {
                display: Display::Block,
                ..default()
            })
            .add_children(&[a, nested, b]);

        app.update();

        let world = app.world();
        let computed_a = world.get::<ComputedNode>(a).unwrap();
        let transform_a = world.get::<UiGlobalTransform>(a).unwrap();
        let computed_b = world.get::<ComputedNode>(b).unwrap();
        let transform_b = world.get::<UiGlobalTransform>(b).unwrap();
        let a_bottom = 0.5 * computed_a.size.y + transform_a.affine().translation.y;
        let b_top = -0.5 * computed_b.size.y + transform_b.affine().translation.y;
        assert!((b_top - a_bottom - 40.).abs() <= 1e-5);
    }

    #[test]
    fn block_layouts_respect_align_content() {
        let mut app = setup_ui_test_app();
        let world = app.world_mut();
        let child = world
            .spawn(Node {
                height: px(20),
                ..default()
            })
            .id();
        world
            .spawn(Node {
                display: Display::Block,
                align_content: AlignContent::End,
                height: px(100),
                ..default()
            })
            .add_child(child);

        app.update();

        assert_eq!(
            app.world()
                .get::<UiGlobalTransform>(child)
                .map(|transform| transform.translation.y),
            Some(90.)
        );
    }

    #[test]
    fn test_border_radius_updates() {
        let mut app = setup_ui_test_app();

        let entity = app
            .world_mut()
            .spawn((Node {
                height: px(100),
                width: px(50),
                ..default()
            },))
            .id();

        app.update();

        let computed = app.world().get::<ComputedNode>(entity).unwrap();
        assert_eq!(computed.border_radius, ResolvedBorderRadius::ZERO);

        app.world_mut()
            .get_mut::<Node>(entity)
            .unwrap()
            .border_radius = BorderRadius::all(px(10));

        app.update();

        let computed = app.world().get::<ComputedNode>(entity).unwrap();
        assert_eq!(
            computed.border_radius,
            ResolvedBorderRadius {
                top_left: Vec2::splat(10.),
                top_right: Vec2::splat(10.),
                bottom_left: Vec2::splat(10.),
                bottom_right: Vec2::splat(10.)
            }
        );

        app.world_mut()
            .get_mut::<Node>(entity)
            .unwrap()
            .border_radius
            .top_left = CornerRadius::circular(vh(30));

        app.update();

        assert_eq!(
            app.world()
                .get::<ComputedNode>(entity)
                .unwrap()
                .border_radius,
            ResolvedBorderRadius {
                top_left: Vec2::splat(TARGET_HEIGHT as f32 * 30. / 100.).min(Vec2::splat(25.)),
                top_right: Vec2::splat(10.),
                bottom_left: Vec2::splat(10.),
                bottom_right: Vec2::splat(10.)
            }
        );

        let border_radius = &mut app
            .world_mut()
            .get_mut::<Node>(entity)
            .unwrap()
            .border_radius;
        border_radius.top_right = CornerRadius::circular(percent(100));
        border_radius.bottom_left = CornerRadius::new(percent(100), percent(100));

        app.update();

        assert_eq!(
            app.world()
                .get::<ComputedNode>(entity)
                .unwrap()
                .border_radius,
            ResolvedBorderRadius {
                top_left: Vec2::splat(TARGET_HEIGHT as f32 * 30. / 100.).min(Vec2::splat(25.)),
                top_right: Vec2::splat(25.),
                bottom_left: Vec2::new(25., 50.),
                bottom_right: Vec2::splat(10.)
            }
        );

        app.world_mut().get_mut::<Node>(entity).unwrap().width = px(200.);

        app.update();

        assert_eq!(
            app.world()
                .get::<ComputedNode>(entity)
                .unwrap()
                .border_radius,
            ResolvedBorderRadius {
                top_left: Vec2::splat(TARGET_HEIGHT as f32 * 30. / 100.).min(Vec2::splat(50.)),
                top_right: Vec2::splat(50.),
                bottom_left: Vec2::new(100., 50.),
                bottom_right: Vec2::splat(10.)
            }
        );

        let world = app.world_mut();
        let mut camera_query = world.query::<&mut Camera>();
        camera_query
            .single_mut(world)
            .unwrap()
            .viewport
            .as_mut()
            .unwrap()
            .physical_size
            .y = TARGET_HEIGHT / 2;

        app.update();

        assert_eq!(
            app.world()
                .get::<ComputedNode>(entity)
                .unwrap()
                .border_radius
                .top_left,
            Vec2::splat(15.)
        );
    }

    #[test]
    fn outlines_relayout_on_outline_removal_and_addition() {
        let mut app = setup_ui_test_app();

        let entity = app
            .world_mut()
            .spawn((
                Node::default(),
                Outline {
                    width: px(10.),
                    offset: px(5.),
                    ..default()
                },
            ))
            .id();

        app.update();

        let computed_node = app.world().get::<ComputedNode>(entity).unwrap();
        assert_eq!(computed_node.outline_width(), 10.);
        assert_eq!(computed_node.outline_offset(), 5.);

        app.world_mut().entity_mut(entity).remove::<Outline>();
        app.update();

        let computed_node = app.world().get::<ComputedNode>(entity).unwrap();
        assert_eq!(computed_node.outline_width(), 0.);
        assert_eq!(computed_node.outline_offset(), 0.);

        app.world_mut().entity_mut(entity).insert(Outline {
            width: px(20.),
            offset: px(10.),
            ..default()
        });
        app.update();

        let computed_node = app.world().get::<ComputedNode>(entity).unwrap();
        assert_eq!(computed_node.outline_width(), 20.);
        assert_eq!(computed_node.outline_offset(), 10.);
    }

    #[test]
    fn ignore_scroll_relayouts_on_removal_and_addition() {
        let mut app = setup_ui_test_app();

        let parent = app
            .world_mut()
            .spawn((
                Node {
                    width: px(100.),
                    height: px(100.),
                    overflow: Overflow::scroll_x(),
                    ..default()
                },
                ScrollPosition(Vec2::new(20., 0.)),
            ))
            .id();
        let child = app
            .world_mut()
            .spawn((
                Node {
                    width: px(200.),
                    height: px(100.),
                    flex_shrink: 0.,
                    ..default()
                },
                IgnoreScroll(BVec2::new(true, false)),
                ChildOf(parent),
            ))
            .id();

        app.update();

        let initial_x = app
            .world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
            .x;

        app.world_mut().entity_mut(child).remove::<IgnoreScroll>();
        app.update();

        assert_eq!(
            app.world()
                .get::<UiGlobalTransform>(child)
                .unwrap()
                .translation
                .x,
            initial_x - 20.
        );

        app.world_mut()
            .entity_mut(child)
            .insert(IgnoreScroll(BVec2::new(true, false)));
        app.update();

        assert_eq!(
            app.world()
                .get::<UiGlobalTransform>(child)
                .unwrap()
                .translation
                .x,
            initial_x
        );
    }

    #[test]
    fn layout_config_relayouts_on_removal_and_addition() {
        let mut app = setup_ui_test_app();

        let entity = app
            .world_mut()
            .spawn((
                Node {
                    width: px(10.5),
                    height: px(10.5),
                    ..default()
                },
                LayoutConfig {
                    use_rounding: false,
                },
            ))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<ComputedNode>(entity).unwrap().size(),
            Vec2::splat(10.5)
        );

        app.world_mut().entity_mut(entity).remove::<LayoutConfig>();
        app.update();

        assert_eq!(
            app.world().get::<ComputedNode>(entity).unwrap().size(),
            Vec2::splat(11.)
        );

        app.world_mut().entity_mut(entity).insert(LayoutConfig {
            use_rounding: false,
        });
        app.update();

        assert_eq!(
            app.world().get::<ComputedNode>(entity).unwrap().size(),
            Vec2::splat(10.5)
        );
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
            assert!(app
                .world()
                .get::<ComputedLayout>(child)
                .is_some_and(ComputedLayout::has_layout));

            app.world_mut().entity_mut(mid).remove::<GhostNode>();
            app.update();
            assert!(!app
                .world()
                .get::<ComputedLayout>(child)
                .is_some_and(ComputedLayout::has_layout));
            assert_eq!(
                app.world().get::<ComputedNode>(child).unwrap().size(),
                Vec2::ZERO
            );

            app.world_mut().entity_mut(mid).insert(GhostNode);
            app.update();
            let root_layout = app
                .world()
                .get::<ComputedLayout>(root)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0;
            let child_layout = app
                .world()
                .get::<ComputedLayout>(child)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0;
            assert_eq!(root_layout.size.width, 100.);
            assert_eq!(child_layout.size.width, 50.);
        }

        #[test]
        fn unparenting_ghost_child_makes_child_layout_root() {
            let mut app = setup_ui_test_app();
            let world = app.world_mut();

            let child = world.spawn(Node::default()).id();
            let ghost = world.spawn(GhostNode).add_child(child).id();
            let root = world.spawn(Node::default()).add_child(ghost).id();

            app.update();
            let world = app.world_mut();
            {
                let mut system_state = bevy_ecs::system::SystemState::<(
                    UiChildren,
                    crate::experimental::UiRootNodes,
                )>::new(world);
                let (ui_children, ui_root_nodes) = system_state.get(world).unwrap();
                assert_eq!(
                    ui_children.iter_ui_children(root).collect::<Vec<_>>(),
                    vec![child]
                );
                assert_eq!(ui_children.get_parent(child), Some(root));
                let root_nodes = ui_root_nodes.iter().collect::<Vec<_>>();
                assert!(root_nodes.contains(&root));
                assert!(!root_nodes.contains(&child));
            }

            world.entity_mut(ghost).detach_all_children();

            app.update();
            let world = app.world_mut();
            let mut system_state = bevy_ecs::system::SystemState::<(
                UiChildren,
                crate::experimental::UiRootNodes,
            )>::new(world);
            let (ui_children, ui_root_nodes) = system_state.get(world).unwrap();
            assert_eq!(
                ui_children.iter_ui_children(root).collect::<Vec<_>>(),
                Vec::<Entity>::new()
            );
            assert_eq!(ui_children.get_parent(child), None);
            let root_nodes = ui_root_nodes.iter().collect::<Vec<_>>();
            assert!(root_nodes.contains(&root));
            assert!(root_nodes.contains(&child));
        }

        #[test]
        fn adding_intermediate_ghost_node_includes_child_in_layout() {
            let mut app = setup_ui_test_app();
            let world = app.world_mut();

            let child = world.spawn(Node::default()).id();
            let mid = world.spawn_empty().add_child(child).id();
            let root = world.spawn(Node::default()).add_child(mid).id();

            app.update();
            let world = app.world_mut();
            {
                let mut system_state = bevy_ecs::system::SystemState::<UiChildren>::new(world);
                let ui_children = system_state.get(world).unwrap();
                assert_eq!(
                    ui_children.iter_ui_children(root).collect::<Vec<_>>(),
                    Vec::<Entity>::new()
                );
            }

            world.entity_mut(mid).insert(GhostNode);

            app.update();
            let world = app.world_mut();
            let mut system_state = bevy_ecs::system::SystemState::<UiChildren>::new(world);
            let ui_children = system_state.get(world).unwrap();
            assert_eq!(
                ui_children.iter_ui_children(root).collect::<Vec<_>>(),
                vec![child]
            );
            assert_eq!(ui_children.get_parent(child), Some(root));
        }

        #[test]
        fn removing_intermediate_ghost_node_excludes_child_from_layout() {
            let mut app = setup_ui_test_app();
            let world = app.world_mut();

            let child = world.spawn(Node::default()).id();
            let mid = world.spawn(GhostNode).add_child(child).id();
            let root = world.spawn(Node::default()).add_child(mid).id();

            app.update();
            let world = app.world_mut();
            {
                let mut system_state = bevy_ecs::system::SystemState::<UiChildren>::new(world);
                let ui_children = system_state.get(world).unwrap();
                assert_eq!(
                    ui_children.iter_ui_children(root).collect::<Vec<_>>(),
                    vec![child]
                );
                assert_eq!(ui_children.get_parent(child), Some(root));
            }

            world.entity_mut(mid).remove::<GhostNode>();

            app.update();
            let world = app.world_mut();
            let mut system_state = bevy_ecs::system::SystemState::<UiChildren>::new(world);
            let ui_children = system_state.get(world).unwrap();
            assert_eq!(
                ui_children.iter_ui_children(root).collect::<Vec<_>>(),
                Vec::<Entity>::new()
            );
        }

        #[test]
        fn fixed_nodes_remain_layout_roots_through_ghost_changes() {
            let mut app = setup_ui_test_app();
            let world = app.world_mut();

            let fixed = world.spawn((Node::default(), FixedNode)).id();
            let ghost1 = world.spawn(GhostNode).add_child(fixed).id();

            app.update();
            let world = app.world_mut();
            {
                let mut system_state = bevy_ecs::system::SystemState::<(
                    crate::experimental::UiRootNodes,
                    Query<Entity, (With<FixedNode>, With<ChildOf>)>,
                )>::new(world);
                let (ui_root_nodes, fixed_nodes) = system_state.get(world).unwrap();
                assert!(ui_root_nodes.iter().collect::<Vec<_>>().contains(&fixed));
                assert!(fixed_nodes.contains(fixed));
            }

            world.spawn(GhostNode).add_child(ghost1);

            app.update();
            let world = app.world_mut();
            {
                let mut system_state = bevy_ecs::system::SystemState::<(
                    crate::experimental::UiRootNodes,
                    Query<Entity, (With<FixedNode>, With<ChildOf>)>,
                )>::new(world);
                let (ui_root_nodes, fixed_nodes) = system_state.get(world).unwrap();
                assert!(ui_root_nodes.iter().collect::<Vec<_>>().contains(&fixed));
                assert!(fixed_nodes.contains(fixed));
            }

            let fixed2 = world.spawn((Node::default(), FixedNode)).id();
            let ghost3 = world.spawn(GhostNode).add_child(fixed2).id();

            app.update();
            let world = app.world_mut();
            {
                let mut system_state = bevy_ecs::system::SystemState::<(
                    crate::experimental::UiRootNodes,
                    Query<Entity, (With<FixedNode>, With<ChildOf>)>,
                )>::new(world);
                let (ui_root_nodes, fixed_nodes) = system_state.get(world).unwrap();
                let root_nodes = ui_root_nodes.iter().collect::<Vec<_>>();
                assert!(root_nodes.contains(&fixed));
                assert!(root_nodes.contains(&fixed2));
                assert!(fixed_nodes.contains(fixed));
                assert!(fixed_nodes.contains(fixed2));
            }

            world.entity_mut(ghost1).detach_all_children();
            world.entity_mut(ghost3).detach_all_children();

            app.update();
            let world = app.world_mut();
            let mut system_state = bevy_ecs::system::SystemState::<(
                crate::experimental::UiRootNodes,
                Query<Entity, (With<FixedNode>, With<ChildOf>)>,
            )>::new(world);
            let (ui_root_nodes, fixed_nodes) = system_state.get(world).unwrap();
            let root_nodes = ui_root_nodes.iter().collect::<Vec<_>>();
            assert!(root_nodes.contains(&fixed));
            assert!(root_nodes.contains(&fixed2));
            assert!(!fixed_nodes.contains(fixed));
            assert!(!fixed_nodes.contains(fixed2));
        }

        #[test]
        fn fixed_ghost_child_is_separate_layout_root() {
            let mut app = setup_ui_test_app();
            let world = app.world_mut();

            let fixed = world.spawn((Node::default(), FixedNode)).id();
            let child = world.spawn(Node::default()).id();
            let ghost = world.spawn(GhostNode).add_children(&[fixed, child]).id();
            let root = world.spawn(Node::default()).add_child(ghost).id();

            app.update();
            let world = app.world_mut();
            let mut system_state = bevy_ecs::system::SystemState::<(
                UiChildren,
                crate::experimental::UiRootNodes,
                Query<Entity, (With<FixedNode>, With<ChildOf>)>,
            )>::new(world);
            let (ui_children, ui_root_nodes, fixed_nodes) = system_state.get(world).unwrap();
            assert_eq!(
                ui_children.iter_ui_children(root).collect::<Vec<_>>(),
                vec![fixed, child]
            );
            assert_eq!(
                ui_children
                    .iter_ui_children(root)
                    .filter(|entity| !fixed_nodes.contains(*entity))
                    .collect::<Vec<_>>(),
                vec![child]
            );
            let root_nodes = ui_root_nodes.iter().collect::<Vec<_>>();
            assert!(root_nodes.contains(&root));
            assert!(!root_nodes.contains(&fixed));
            assert!(fixed_nodes.contains(fixed));
        }

        #[test]
        fn unghost_ghost_node_with_fixed_and_normal_children() {
            let mut app = setup_ui_test_app();
            let world = app.world_mut();

            let fixed = world.spawn((Node::default(), FixedNode)).id();
            let child = world.spawn(Node::default()).id();
            let ghost = world.spawn(GhostNode).add_children(&[fixed, child]).id();

            app.update();
            let world = app.world_mut();
            {
                let mut system_state = bevy_ecs::system::SystemState::<(
                    crate::experimental::UiRootNodes,
                    Query<Entity, (With<FixedNode>, With<ChildOf>)>,
                )>::new(world);
                let (ui_root_nodes, fixed_nodes) = system_state.get(world).unwrap();
                let root_nodes = ui_root_nodes.iter().collect::<Vec<_>>();
                assert!(root_nodes.contains(&fixed));
                assert!(root_nodes.contains(&child));
                assert!(fixed_nodes.contains(fixed));
            }

            world
                .entity_mut(ghost)
                .remove::<GhostNode>()
                .insert(Node::default());

            app.update();
            let world = app.world_mut();
            let mut system_state = bevy_ecs::system::SystemState::<(
                UiChildren,
                crate::experimental::UiRootNodes,
                Query<Entity, (With<FixedNode>, With<ChildOf>)>,
            )>::new(world);
            let (ui_children, ui_root_nodes, fixed_nodes) = system_state.get(world).unwrap();
            let root_nodes = ui_root_nodes.iter().collect::<Vec<_>>();
            assert!(root_nodes.contains(&ghost));
            assert!(!root_nodes.contains(&child));
            assert!(fixed_nodes.contains(fixed));
            assert_eq!(
                ui_children
                    .iter_ui_children(ghost)
                    .filter(|entity| !fixed_nodes.contains(*entity))
                    .collect::<Vec<_>>(),
                vec![child]
            );
        }

        #[test]
        fn removing_and_replacing_intermediate_ghost_should_relayout_parent() {
            let mut app = setup_ui_test_app();

            let world = app.world_mut();
            let child = world
                .spawn(Node {
                    width: px(50.),
                    height: px(30.),
                    ..default()
                })
                .id();
            let ghost = world.spawn(GhostNode).add_child(child).id();
            let root = world.spawn(Node::default()).add_child(ghost).id();
            app.update();

            app.world_mut().entity_mut(ghost).remove::<GhostNode>();

            app.update();

            assert!(app
                .world()
                .entity(root)
                .get::<ComputedNode>()
                .unwrap()
                .size()
                .abs_diff_eq(Vec2::ZERO, 1e-5));
            app.world_mut().entity_mut(ghost).insert(GhostNode);

            app.update();

            assert!(app
                .world()
                .entity(root)
                .get::<ComputedNode>()
                .unwrap()
                .size()
                .abs_diff_eq(Vec2::new(50., 30.), 1e-5));
        }
    }
}
