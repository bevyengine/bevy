use crate::{
    layout_tree::{compute_layout, TaffyStyle},
    ui_transform::{UiGlobalTransform, UiTransform},
    ComputedNode, ComputedUiRenderTargetInfo, ContentSize, Display, FixedNode, GhostNode,
    IgnoreScroll, LayoutConfig, Node, Outline, OverflowAxis, OverrideClip, ScrollPosition,
};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With, Without},
    resource::Resource,
    system::{Local, ParamSet, Query, Res, ResMut},
    world::Ref,
};

use bevy_log::warn_once;
use bevy_math::{Affine2, Vec2};
use bevy_sprite::BorderRect;
use layout_tree::ComputedLayout;
use thiserror::Error;

use bevy_text::{ComputedTextBlock, EmSize, FontCx, RemSize, TextFont, DEFAULT_REM_SIZE_PX};

pub mod clipping;
mod convert;
pub mod debug;
pub mod layout_tree;
#[cfg(test)]
mod tests;

/// `UiTreeDirty` is used to signal that a `Node` 's subtree contains a
/// change that requires a layout update.
/// ZST marker component uses change detection to signal changes.
///
/// Doesn't need to be reset,
/// Optimization copied from `bevy_transform`'s `TransformTreeChanged`.
#[derive(Component, Default, Debug, Clone)]
pub struct UiTreeDirty;

/// List of all UI root nodes.
/// Updated at start of UI schedule in `PostLayout` by `update_ui_roots`.
#[derive(Resource, Default)]
pub struct UiRoots {
    /// All parentless UI nodes.
    parentless: Vec<Entity>,
    /// All parentless, non-ghost root UI nodes.    
    parentless_non_ghosts: Vec<Entity>,
    /// All non-ghost nodes with no non-ghost ancestors.
    roots_under_ghosts: Vec<Entity>,
    /// All ghost nodes with all ghost ancestors.
    root_ghosts: Vec<Entity>,
    /// All valid fixed nodes (parented, non-ghost, all ancestors are UI nodes).
    fixed_nodes: Vec<Entity>,
}

impl UiRoots {
    /// Returns the root node where layout updates start from, with [`GhostNode`]s flattened.
    /// Includes parentless non-ghost nodes, non-ghost nodes with only ghost ancestors,
    /// and valid [`FixedNode`]s.
    pub fn layout_roots(&self) -> impl Iterator<Item = Entity> {
        self.parentless_non_ghosts
            .iter()
            .chain(self.roots_under_ghosts.iter())
            .chain(self.fixed_nodes.iter())
            .copied()
    }

    /// Returns the nodes where geometry updates start from.
    /// Includes all parentless UI nodes, including [`GhostNode`]s, and valid [`FixedNode`]s.
    /// Starting at root ghosts allows their transforms to propagate to descendants.
    pub fn geometry_roots(&self) -> impl Iterator<Item = Entity> {
        self.parentless
            .iter()
            .chain(self.fixed_nodes.iter())
            .copied()
    }
}

impl UiRoots {
    fn clear(&mut self) {
        self.parentless.clear();
        self.parentless_non_ghosts.clear();
        self.roots_under_ghosts.clear();
        self.root_ghosts.clear();
        self.fixed_nodes.clear();
    }
}

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
    #[error("UI root entity is missing or doesn't have the components needed for layout.")]
    InvalidUiRoot,
}

/// Update the list of root nodes
pub fn update_ui_roots(
    mut navigation_stack: Local<Vec<Entity>>,
    mut ui_roots: ResMut<UiRoots>,
    all_roots_query: Query<Entity, (With<Node>, Without<ChildOf>)>,
    roots_query: Query<Entity, (With<Node>, Without<ChildOf>, Without<GhostNode>)>,
    fixed_nodes_query: Query<(Entity, &ChildOf), (With<FixedNode>, Without<GhostNode>)>,
    fixed_nodes_ancestor_query: Query<Option<&ChildOf>, With<Node>>,
    ghost_roots_query: Query<(Entity, Option<&Children>), (With<GhostNode>, Without<ChildOf>)>,
    flattening_query: Query<(Entity, Has<GhostNode>, Option<&Children>), With<Node>>,
) {
    ui_roots.clear();
    ui_roots.parentless.extend(all_roots_query.iter());
    ui_roots.parentless_non_ghosts.extend(roots_query.iter());

    for (ghost_root, maybe_children) in &ghost_roots_query {
        ui_roots.root_ghosts.push(ghost_root);
        if let Some(children) = maybe_children {
            navigation_stack.extend(children);
            while let Some(entity) = navigation_stack.pop() {
                let Ok((entity, is_ghost, maybe_children)) = flattening_query.get(entity) else {
                    continue;
                };
                if is_ghost {
                    ui_roots.root_ghosts.push(entity);
                    if let Some(children) = maybe_children {
                        navigation_stack.extend(children);
                    }
                } else if !fixed_nodes_query.contains(entity) {
                    ui_roots.roots_under_ghosts.push(entity);
                }
            }
        }
    }

    for (entity, child_of) in &fixed_nodes_query {
        let mut ancestor = Some(child_of.parent());
        let mut is_valid_fixed_node = true;
        while let Some(ancestor_entity) = ancestor {
            let Ok(maybe_child_of) = fixed_nodes_ancestor_query.get(ancestor_entity) else {
                is_valid_fixed_node = false;
                break;
            };
            ancestor = maybe_child_of.map(ChildOf::parent);
        }

        if is_valid_fixed_node {
            ui_roots.fixed_nodes.push(entity);
        }
    }
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

/// Sync each `Node` with its corresponding `TaffyStyle`.
pub fn sync_taffy_styles_with_nodes(
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

/// Clear the local dirty flags that are only valid for the current frame.
pub fn clear_transient_dirty_flags(mut computed_layout_query: Query<&mut ComputedLayout>) {
    computed_layout_query
        .par_iter_mut()
        .for_each(|mut computed_layout| {
            computed_layout
                .bypass_change_detection()
                .clear_transient_dirty_flags();
        });
}

/// Identify entities whose UI layout input components have been changed, added or removed.
/// Mark their `UiTreeDirty` component changed, then walk up the tree and mark
/// each ancestor's `UiTreeDirty` changed.
pub fn mark_dirty_ui_trees(
    changed_ui_components_query: Query<
        Entity,
        (
            Or<(
                Changed<TaffyStyle>,
                Changed<ContentSize>,
                Changed<UiTransform>,
                Changed<ScrollPosition>,
                Changed<Outline>,
                Changed<LayoutConfig>,
                Changed<IgnoreScroll>,
                Changed<Children>,
                Changed<ChildOf>,
                Added<FixedNode>,
                Added<GhostNode>,
                Added<OverrideClip>,
            )>,
            With<Node>,
        ),
    >,
    mut removed_outlines: RemovedComponents<Outline>,
    mut removed_layout_configs: RemovedComponents<LayoutConfig>,
    mut removed_ignore_scrolls: RemovedComponents<IgnoreScroll>,
    mut removed_fixed_nodes: RemovedComponents<FixedNode>,
    mut removed_child_ofs: RemovedComponents<ChildOf>,
    mut removed_nodes: RemovedComponents<Node>,
    mut removed_ghost_nodes: RemovedComponents<GhostNode>,
    mut removed_override_clip: RemovedComponents<OverrideClip>,
    mut trees: Query<(
        &mut UiTreeDirty,
        Has<FixedNode>,
        Has<GhostNode>,
        Option<&ChildOf>,
    )>,
) {
    let removed = removed_outlines
        .read()
        .chain(removed_layout_configs.read())
        .chain(removed_ignore_scrolls.read())
        .chain(removed_fixed_nodes.read())
        .chain(removed_child_ofs.read())
        .chain(removed_nodes.read())
        .chain(removed_ghost_nodes.read())
        .chain(removed_override_clip.read());

    for mut next in changed_ui_components_query.iter().chain(removed) {
        while let Ok((mut dirty_tree, is_fixed_node, is_ghost_node, maybe_child_of)) =
            trees.get_mut(next)
        {
            // If `UiDirtyTree` was added since the last update, `is_changed()` will be `true` even if this node wasn't already visited.
            // So we can't skip it as we don't know if it was already visited.
            if dirty_tree.is_changed() && !dirty_tree.is_added() {
                break;
            }
            dirty_tree.set_changed();
            // Since `FixedNode`s create a new layout context, changes to a `FixedNode` or its descendants do not affect their ancestors.
            // So abort upwards dirty tree propagation.
            // A ghost node cannot also be fixed, so `FixedNode` is ignored if `GhostNode` is present.
            if is_fixed_node && !is_ghost_node {
                break;
            }
            let Some(child_of) = maybe_child_of else {
                // Reached UI root
                break;
            };
            next = child_of.0;
        }
    }
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
pub fn ui_layout_system(
    ui_roots: Res<UiRoots>,
    ui_children: Query<(Option<&Children>, Has<GhostNode>, Ref<UiTreeDirty>), With<Node>>,
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
            Has<OverrideClip>,
            Ref<UiTreeDirty>,
        ),
        With<Node>,
    >,
    style_query: Query<&'static TaffyStyle>,
    mut node_queries: ParamSet<(
        Query<&mut ComputedLayout>,
        Query<(
            &mut ComputedNode,
            &mut UiGlobalTransform,
            &mut ComputedLayout,
            Has<GhostNode>,
        )>,
    )>,
    mut buffer_query: Query<&mut ComputedTextBlock>,
    mut font_system: ResMut<FontCx>,
    added_fixed_node_query: Query<Entity, Added<FixedNode>>,
    mut removed_fixed_nodes: RemovedComponents<FixedNode>,
    (
        tree_changed_query,
        mut removed_child_ofs,
        mut removed_nodes,
        added_ghost_nodes,
        mut removed_ghost_nodes,
    ): (
        Query<Entity, Or<(Changed<Children>, Changed<ChildOf>, Added<Node>)>>,
        RemovedComponents<ChildOf>,
        RemovedComponents<Node>,
        Query<Entity, Added<GhostNode>>,
        RemovedComponents<GhostNode>,
    ),
    (mut child_stack, mut root_stack, mut fixed_node_changes, mut ghost_stack): (
        Local<Vec<taffy::NodeId>>,
        Local<Vec<taffy::NodeId>>,
        Local<Vec<Entity>>,
        Local<Vec<Entity>>,
    ),
) {
    // Using a vec to track their changes since `FixedNode`s should be rare, and rarely updated.
    fixed_node_changes.clear();
    fixed_node_changes.extend(
        added_fixed_node_query
            .iter()
            .chain(removed_fixed_nodes.read()),
    );

    // Reachability only changes when the tree does. On those updates we do a full walk from each UI root node,
    // setting `ComputedLayout::reached` to true for every node encountered on the walk.
    // Unreached nodes, `Node` entities with a non-`Node` ancestor, are cleared at the end of this system.
    // Otherwise clean subtrees are skipped and their `reached` flags left unchanged.
    // This could be done incrementally, but it would add a lot of extra complexity and the walk is relatively cheap.
    let tree_changed = !tree_changed_query.is_empty()
        || !removed_child_ofs.is_empty()
        || !removed_nodes.is_empty();
    let ghosts_changed = !added_ghost_nodes.is_empty() || !removed_ghost_nodes.is_empty();
    let needs_full_walk = tree_changed || ghosts_changed || !fixed_node_changes.is_empty();

    removed_child_ofs.clear();
    removed_nodes.clear();
    removed_ghost_nodes.clear();
    removed_fixed_nodes.clear();

    root_stack.clear();
    ghost_stack.clear();

    let mut computed_layout_query = node_queries.p0();
    for ui_root_entity in ui_roots.layout_roots() {
        let Ok(target) = target_query.get(ui_root_entity) else {
            continue;
        };

        if compute_layout(
            ui_root_entity,
            target.physical_size(),
            &ui_children,
            &node_query,
            &style_query,
            &mut computed_layout_query,
            &fixed_node_changes,
            &mut buffer_query,
            &mut font_system,
            &mut child_stack,
            needs_full_walk,
            &mut ghost_stack,
        )
        .is_err()
        {
            warn_once!("Invalid UI root entity: {ui_root_entity}.");
        }

        child_stack.clear();
    }

    // Finish if there weren't any changes that might have changed the entity hierarchy.
    if !needs_full_walk {
        return;
    }

    // `GhostNode`s are stepped over during layout, so need to mark them separately as live UI nodes
    // so they aren't cleared below.
    for ghost_node in ghost_stack.iter().chain(ui_roots.root_ghosts.iter()) {
        if let Ok(mut computed_layout) = computed_layout_query.get_mut(*ghost_node) {
            let computed_layout = computed_layout.bypass_change_detection();
            computed_layout.clear();
            computed_layout.set_reached(true);
        }
    }

    // Clear any UI node entities that became unreachable due to hierarchy changes.
    node_queries.p1().par_iter_mut().for_each(
        |(mut node, mut global_transform, mut computed_layout, is_ghost)| {
            let reached = computed_layout.reached();
            if !reached {
                computed_layout.clear();
            }
            computed_layout.set_reached(false);

            if (is_ghost && reached) || computed_layout.has_layout() {
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
    ui_roots: Res<UiRoots>,
    rem_size: Res<RemSize>,
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
        Has<FixedNode>,
        Has<GhostNode>,
        Ref<UiTreeDirty>,
        Option<&Children>,
    )>,
    mut child_stack: Local<Vec<Entity>>,
) {
    for ui_root_entity in ui_roots.geometry_roots() {
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

fn update_uinode_geometry_recursive(
    root: Entity,
    entity: Entity,
    inherited_use_rounding: bool,
    target_size: Vec2,
    mut inherited_transform: Affine2,
    computed_nodes_query: &mut Query<(
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
        Has<FixedNode>,
        Has<GhostNode>,
        Ref<UiTreeDirty>,
        Option<&Children>,
    )>,
    inverse_target_scale_factor: f32,
    parent_size: Vec2,
    parent_scroll_position: Vec2,
    rem_size: RemSize,
    child_stack: &mut Vec<Entity>,
    force_update: bool,
) {
    if let Ok((
        mut computed_node,
        transform,
        mut global_transform,
        style,
        computed_layout,
        em_size,
        maybe_layout_config,
        maybe_outline,
        maybe_scroll_position,
        maybe_scroll_sticky,
        is_fixed_node,
        is_ghost_node,
        tree_changed,
        maybe_children,
    )) = computed_nodes_query.get_mut(entity)
    {
        // We skip any non-root `FixedNode`s, otherwise they would get updated twice.
        // Unless they are `GhostNode`s since `GhostNode` overrides `FixedNode`.
        if is_fixed_node && !is_ghost_node && root != entity {
            return;
        }

        if !force_update
            && !tree_changed.is_changed()
            && !computed_layout.layout_dirty()
            && !computed_layout.subtree_dirty()
        {
            return;
        }

        // A `GhostNode`'s `ComputedNode` is cleared except border radius (doesn't matter as resolved border radius is always zero for zero-sized nodes),
        // scale factor and em and rem sizes.
        // The inherited base values are just passed through to the child.
        if is_ghost_node {
            computed_node.set_if_neq(ComputedNode {
                border_radius: computed_node.border_radius,
                inverse_scale_factor: inverse_target_scale_factor,
                em_size: *em_size,
                rem_size,
                ..ComputedNode::DEFAULT
            });

            inherited_transform *= transform.compute_affine(
                inverse_target_scale_factor.recip(),
                // Normally percentage translations are resolved based on a node's own size
                // but the size of a `GhostNode` is always zero.
                // Instead for a `GhostNode` percentage translations are resolved using the size
                // of its parent.
                parent_size,
                target_size,
                *em_size,
                rem_size,
            );

            if inherited_transform != **global_transform {
                *global_transform = inherited_transform.into();
            }

            if let Some(children) = maybe_children {
                let start = child_stack.len();
                child_stack.extend(children);
                let end = child_stack.len();
                let inherited_force_update = force_update || tree_changed.is_changed();
                for child_index in start..end {
                    update_uinode_geometry_recursive(
                        root,
                        child_stack[child_index],
                        inherited_use_rounding,
                        target_size,
                        inherited_transform,
                        computed_nodes_query,
                        inverse_target_scale_factor,
                        parent_size,
                        parent_scroll_position,
                        rem_size,
                        child_stack,
                        inherited_force_update,
                    );
                }
                child_stack.truncate(start);
            }
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
        if computed_node.size != layout_size
            || computed_node.unrounded_size != unrounded_size
            || computed_node.inverse_scale_factor != inverse_target_scale_factor
        {
            computed_node.size = layout_size;
            computed_node.unrounded_size = unrounded_size;
            computed_node.inverse_scale_factor = inverse_target_scale_factor;
        }

        let content_size = Vec2::new(
            layout.scrollable_overflow_rect.right,
            layout.scrollable_overflow_rect.bottom,
        );
        if computed_node.content_size != content_size {
            computed_node.content_size = content_size;
        }

        let taffy_rect_to_border_rect = |rect: taffy::Rect<f32>| BorderRect {
            min_inset: Vec2::new(rect.left, rect.top),
            max_inset: Vec2::new(rect.right, rect.bottom),
        };

        let new_border = taffy_rect_to_border_rect(layout.border);
        if computed_node.border != new_border {
            computed_node.border = new_border;
        }
        let new_padding = taffy_rect_to_border_rect(layout.padding);
        if computed_node.padding != new_padding {
            computed_node.padding = new_padding;
        }

        if computed_node.em_size != *em_size {
            computed_node.em_size = *em_size;
        }
        if computed_node.rem_size != rem_size {
            computed_node.rem_size = rem_size;
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
                        computed_node.size().x,
                        target_size,
                        *em_size,
                        rem_size,
                    )
                    .unwrap_or(0.)
                    .max(0.)
            } else {
                0.
            };

            if computed_node.outline_width != new_outline_width {
                computed_node.outline_width = new_outline_width;
            }

            let new_outline_offset = outline
                .offset
                .resolve(
                    inverse_target_scale_factor.recip(),
                    computed_node.size().x,
                    target_size,
                    *em_size,
                    rem_size,
                )
                .unwrap_or(0.)
                // Clamp outline offsets to at least the length of the node's shorter side
                // Negative offset outlines can be useful to create thing like in-set focus indicators
                .max(-0.5 * computed_node.size.min_element());
            if computed_node.outline_offset != new_outline_offset {
                computed_node.outline_offset = new_outline_offset;
            }
        } else if computed_node.outline_width != 0. || computed_node.outline_offset != 0. {
            computed_node.outline_width = 0.;
            computed_node.outline_offset = 0.;
        }

        let new_scrollbar_size =
            Vec2::new(layout.scrollbar_size.width, layout.scrollbar_size.height);
        if computed_node.scrollbar_size != new_scrollbar_size {
            computed_node.scrollbar_size = new_scrollbar_size;
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

        let clamped_scroll_position = scroll_position.clamp(
            Vec2::ZERO,
            Vec2::new(layout.scroll_width(), layout.scroll_height()),
        );

        let physical_scroll_position = clamped_scroll_position.floor();

        if computed_node.scroll_position != physical_scroll_position {
            computed_node.scroll_position = physical_scroll_position;
        }

        if let Some(children) = maybe_children {
            let start = child_stack.len();
            child_stack.extend(children);
            let end = child_stack.len();

            let inherited_force_update =
                force_update || computed_layout.layout_dirty() || computed_layout.self_dirty();
            for child_index in start..end {
                update_uinode_geometry_recursive(
                    root,
                    child_stack[child_index],
                    use_rounding,
                    target_size,
                    inherited_transform,
                    computed_nodes_query,
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
