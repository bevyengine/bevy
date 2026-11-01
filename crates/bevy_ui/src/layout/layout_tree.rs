use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::{Entity, EntityHashSet},
    hierarchy::Children,
    query::{Has, With},
    system::Query,
    world::Ref,
};
use bevy_math::{UVec2, Vec2};
use bevy_text::{ComputedTextBlock, FontCx};
use bevy_utils::default;
use taffy::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_leaf_layout, compute_oof_layout, compute_root_layout,
    round_layout,
    style::{AvailableSpace, Display, Style},
    Cache, CacheTree, Layout, LayoutBlockContainer, LayoutContainingBlock, LayoutFlexboxContainer,
    LayoutGridContainer, LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, RoundTree, RunMode,
    TraversePartialTree, TraverseTree,
};

use crate::{
    layout::UiTreeDirty, ContentSize, FixedNode, GhostNode, IgnoreScroll, LayoutConfig,
    LayoutError, Measure, MeasureArgs, Node, NodeMeasure, Outline, OverrideClip, ScrollPosition,
    UiTransform,
};

#[expect(
    unsafe_code,
    reason = "taffy::Style is only thread-unsafe with the calc feature"
)]
// SAFETY: taffy::Style is only thread-unsafe with the calc feature
unsafe impl Send for TaffyStyle {}

#[expect(
    unsafe_code,
    reason = "taffy::Style is only thread-unsafe with the calc feature"
)]
// SAFETY: taffy::Style is only thread-unsafe with the calc feature
unsafe impl Sync for TaffyStyle {}

#[derive(Default, Component, Clone, Deref, DerefMut)]
pub struct TaffyStyle(pub Style);

pub static VIEWPORT_NODE_TAFFY_STYLE: TaffyStyle = TaffyStyle(Style {
    dummy: core::marker::PhantomData,
    display: Display::Grid,
    item_is_table: false,
    item_is_replaced: false,
    item_is_compressible_replaced: false,
    box_sizing: taffy::BoxSizing::BorderBox,
    direction: taffy::Direction::Ltr,
    overflow: taffy::Point {
        x: taffy::Overflow::Visible,
        y: taffy::Overflow::Visible,
    },
    scrollbar_width: 0.0,
    contain: taffy::Contain::NONE,
    position: taffy::Position::Relative,
    inset: taffy::Rect::auto(),
    size: taffy::Size {
        width: taffy::Dimension::percent(1.0),
        height: taffy::Dimension::percent(1.0),
    },
    min_size: taffy::Size::auto(),
    max_size: taffy::Size::auto(),
    aspect_ratio: None,
    margin: taffy::Rect::zero(),
    padding: taffy::Rect::zero(),
    border: taffy::Rect::zero(),
    align_items: taffy::AlignItems::START,
    align_self: None,
    justify_items: taffy::JustifyItems::START,
    justify_self: None,
    align_content: taffy::AlignContent::NORMAL,
    justify_content: taffy::AlignContent::NORMAL,
    gap: taffy::Size::zero(),
    text_align: taffy::TextAlign::Auto,
    flex_direction: taffy::FlexDirection::Row,
    flex_wrap: taffy::FlexWrap::NoWrap,
    flex_basis: taffy::Dimension::auto(),
    flex_grow: 0.0,
    flex_shrink: 1.0,
    grid_template_rows: Vec::new(),
    grid_template_columns: Vec::new(),
    grid_auto_rows: Vec::new(),
    grid_auto_columns: Vec::new(),
    grid_auto_flow: taffy::GridAutoFlow::Row,
    grid_template_areas: None,
    grid_template_column_names: Vec::new(),
    grid_template_row_names: Vec::new(),
    grid_row: taffy::Line {
        start: taffy::GridPlacement::Span(1),
        end: taffy::GridPlacement::Auto,
    },
    grid_column: taffy::Line {
        start: taffy::GridPlacement::Span(1),
        end: taffy::GridPlacement::Auto,
    },
});

/// `NodeId`s wrap a `u64` that can be directly converted to and from an `Entity`.
/// We create and assign the `NodeId`s ourselves in order that each `NodeId` corresponds to an `Entity`.
pub(super) const fn entity_node_id(entity: Entity) -> NodeId {
    NodeId::new(entity.to_bits())
}

/// `entity.to_bits()` can't be zero, so we use a `NodeId` of zero to represent viewport nodes.
pub const VIEWPORT_NODE_ID: NodeId = NodeId::new(0u64);

/// `NodeId`s wrap a `u64` that can be directly converted to and from an `Entity`.
/// We create and assign the `NodeId`s ourselves in order that each `NodeId` corresponds to an `Entity`.
/// Returns None if the `NodeId` is the viewport, which doesn't correspond to an entity.
pub(super) fn entity_from_node_id(node_id: NodeId) -> Option<Entity> {
    (node_id != VIEWPORT_NODE_ID).then(|| Entity::from_bits(u64::from(node_id)))
}

/// Collect the children of `parent`, after flattening any `GhostNode`s, that are valid UI nodes that should be exposed to Taffy.
///
/// Returns true if any of the flattened `GhostNode`s is marked dirty.
pub(super) fn collect_ui_children(
    parent: Entity,
    ui_children: &Query<(Option<&Children>, Has<GhostNode>, Ref<UiTreeDirty>), With<Node>>,
    child_stack: &mut Vec<NodeId>,
    ghost_stack: &mut Vec<Entity>,
) -> bool {
    let Ok((children, _, _)) = ui_children.get(parent) else {
        return false;
    };

    let mut dirty_ghost = false;
    for &child in children.into_iter().flatten() {
        let Ok((_, is_ghost, tree_changed)) = ui_children.get(child) else {
            continue;
        };
        if is_ghost {
            ghost_stack.push(child);
            dirty_ghost |= collect_ui_children(child, ui_children, child_stack, ghost_stack)
                || tree_changed.is_changed();
        } else {
            child_stack.push(entity_node_id(child));
        }
    }

    dirty_ghost
}

/// Cached and computed layout state for a UI node.
#[derive(Component, Debug, Clone, Default)]
pub struct ComputedLayout {
    /// The unrounded layout for this node returned from Taffy.
    unrounded: Option<Layout>,
    /// The rounded layout for this node returned from Taffy.
    rounded: Option<Layout>,
    /// Cached sizing results used during layout by Taffy.
    cache: Cache,
    /// This UI node was reached from a layout root by the last full walk
    reached_in_full_walk: bool,
    /// This UI node is a layout root. That is either a parentless root UI node, a UI node with only `GhostNode` ancestors, or a `FixedNode`.
    is_layout_root: bool,
    /// List of this UI Node's children (either directly or transitively via `GhostNode`s) that are also valid UI nodes. Non-UI nodes shouldn't be in this list.
    ui_children: Vec<NodeId>,
    /// List of this UI Nodes's out-of-flow children (absolute or fixed positioned)
    hoisted_children: Vec<NodeId>,
    /// If true, the layout returned from `Taffy` changed since the last frame.
    /// Set to `false` each frame in `UiSystems::Prepare` by the `clear_transient_dirty_flags` system.
    layout_dirty: bool,
    /// If `true` local inputs have changed since the last frame.
    /// Set to `false` each frame in `UiSystems::Prepare` by the `clear_transient_dirty_flags` system.
    self_dirty: bool,
    /// If `true` this node or its descendants geometry needs to be updated.
    /// Set to `false` each frame in `UiSystems::Prepare` by the `clear_transient_dirty_flags` system.
    subtree_dirty: bool,
    /// True if the node has an `Outline` component.
    has_outline: bool,
    /// True if node has a `LayoutConfig` component.
    has_layout_config: bool,
    /// True if node has an `IgnoreScroll` component.
    has_ignore_scroll: bool,
    /// True if node has an `OverrideClip` component.
    has_override_clip: bool,
}

impl ComputedLayout {
    /// Clear all state
    pub fn clear(&mut self) {
        self.unrounded = None;
        self.rounded = None;
        self.cache.clear();
        self.ui_children.clear();
        self.hoisted_children.clear();
        self.reached_in_full_walk = false;
        self.is_layout_root = false;
        self.layout_dirty = false;
        self.subtree_dirty = false;
        self.self_dirty = false;
        self.has_outline = false;
        self.has_layout_config = false;
        self.has_ignore_scroll = false;
        self.has_override_clip = false;
    }

    /// Returns true if both rounded and unrounded layouts are present
    #[inline]
    pub const fn has_layout(&self) -> bool {
        self.unrounded.is_some() && self.rounded.is_some()
    }

    /// True if the layout output by taffy was changed and the derived components, like `ComputedNode` and `CalculatedClip`, need to be updated.
    #[inline]
    pub const fn layout_dirty(&self) -> bool {
        self.layout_dirty
    }

    /// True if this node or its descendants needs a geometry update.
    #[inline]
    pub const fn subtree_dirty(&self) -> bool {
        self.subtree_dirty
    }

    /// True if local inputs changed such as `UiTransform` and `ScrollPosition`.
    #[inline]
    pub const fn self_dirty(&self) -> bool {
        self.self_dirty
    }

    /// Set rounded layout
    pub fn set_rounded(&mut self, layout: Layout) {
        if self.rounded == Some(layout) {
            return;
        }
        self.layout_dirty = true;
        self.rounded = Some(layout);
    }

    /// Returns `true` if the layout changed since the last update
    pub fn set_unrounded(&mut self, layout: Layout) -> bool {
        if self.unrounded == Some(layout) {
            return false;
        }
        self.layout_dirty = true;
        self.unrounded = Some(layout);
        true
    }

    /// Set whether the last full walk reached this node from a layout root
    #[inline]
    pub(super) const fn set_reached(&mut self, reached: bool) {
        self.reached_in_full_walk = reached;
    }

    /// Returns true if the last full walk reached this node from a layout root
    #[inline]
    pub(super) const fn reached(&self) -> bool {
        self.reached_in_full_walk
    }

    /// Returns the rounded or unrounded layout for the node, and its unrounded size.
    pub fn get_layout(&self, use_rounding: bool) -> Option<(Layout, Vec2)> {
        let unrounded = self.unrounded?;
        let selected_layout = if use_rounding {
            self.rounded?
        } else {
            unrounded
        };
        let unrounded_size = Vec2::new(unrounded.size.width, unrounded.size.height);

        Some((selected_layout, unrounded_size))
    }

    /// Get the UI children for this Node, by `NodeId`.
    /// This is the subset of a `Node` entity's children that also have a `Node` component and should be visible to `Taffy`,
    /// merged with any `Node`'s hoisted up to replace a [`GhostNode`] ancestor.
    #[inline]
    pub fn child_nodes(&self) -> &[NodeId] {
        &self.ui_children
    }

    /// Get the UI children for this Node, by `Entity`.
    /// This is the subset of a `Node` entity's children that also have a `Node` component and should be visible to `Taffy`,
    /// merged with any `Node`'s hoisted up to replace a [`GhostNode`] ancestor.
    #[inline]
    pub fn child_entities(&self) -> impl Iterator<Item = Entity> {
        self.ui_children.iter().map(|node_id| {
            entity_from_node_id(*node_id).expect("Viewport Node found in UI children.")
        })
    }

    /// True if this UI node is a layout root.
    /// A layout root has the `Node` component and is either parentless, only has `GhostNode` ancestors, or is a `FixedNode`.
    #[inline]
    pub fn is_layout_root(&self) -> bool {
        self.is_layout_root
    }

    /// Clear dirty flags that are only valid for the current frame.
    #[inline]
    pub(super) const fn clear_transient_dirty_flags(&mut self) {
        self.layout_dirty = false;
        self.self_dirty = false;
        self.subtree_dirty = false;
    }
}

/// Compute and store layout results for one UI root entity.
pub fn compute_layout<'w, 's>(
    ui_root_entity: Entity,
    render_target_resolution: UVec2,
    ui_children: &Query<(Option<&Children>, Has<GhostNode>, Ref<UiTreeDirty>), With<Node>>,
    node_query: &Query<
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
    style_query: &Query<'w, 's, &'static TaffyStyle>,
    computed_layout_query: &mut Query<'w, 's, &'static mut ComputedLayout>,
    fixed_node_changes: &EntityHashSet,
    buffer_query: &mut Query<&mut ComputedTextBlock>,
    font_system: &mut FontCx,
    child_stack: &mut Vec<NodeId>,
    needs_full_walk: bool,
    ghost_stack: &mut Vec<Entity>,
) -> Result<(), LayoutError> {
    let Some(subtree_state) = sync_layout_tree(
        ui_root_entity,
        ui_root_entity,
        ui_children,
        node_query,
        computed_layout_query,
        fixed_node_changes,
        child_stack,
        needs_full_walk,
        ghost_stack,
    ) else {
        return Err(LayoutError::InvalidUiRoot);
    };

    if subtree_state != SubtreeState::LayoutDirty {
        return Ok(());
    }

    let root_node_id = entity_node_id(ui_root_entity);

    let available_space = taffy::Size {
        width: AvailableSpace::Definite(render_target_resolution.x as f32),
        height: AvailableSpace::Definite(render_target_resolution.y as f32),
    };

    {
        let mut measure_function = |known_dimensions: taffy::Size<Option<f32>>,
                                    available_space: taffy::Size<AvailableSpace>,
                                    entity: Entity,
                                    style: &Style| {
            let Ok((_, content_size, ..)) = node_query.get(entity) else {
                return taffy::Size::ZERO;
            };
            let Some(measure) = content_size.measure.as_ref() else {
                return taffy::Size::ZERO;
            };
            let mut measure_args = MeasureArgs {
                known_width: known_dimensions.width,
                known_height: known_dimensions.height,
                available_width: available_space.width,
                available_height: available_space.height,
                font_system,
                buffer: None,
                style,
            };
            let buffer = get_text_buffer(
                crate::widget::TextMeasure::needs_buffer(
                    measure_args.resolve_width().effective,
                    measure_args.resolve_height().effective,
                    available_space.width,
                ),
                measure,
                buffer_query,
            );
            measure_args.buffer = buffer;
            let size = measure.measure(measure_args);
            taffy::Size {
                width: size.x,
                height: size.y,
            }
        };

        let mut tree = UiLayoutTree {
            style_query,
            computed_layout_query,
            viewport_layout: ComputedLayout {
                ui_children: vec![root_node_id],
                ..default()
            },
            measure_function: &mut measure_function,
            layout_changed: false,
        };

        compute_root_layout(&mut tree, VIEWPORT_NODE_ID, available_space);
        if tree.layout_changed || needs_full_walk {
            round_layout(&mut tree, VIEWPORT_NODE_ID);
        }
    };

    Ok(())
}

/// The current state of a UI subtree and what needs to be updated.
#[derive(Clone, Copy, Default, PartialEq)]
enum SubtreeState {
    /// Nothing in this subtree needs to be updated.
    #[default]
    Clean,
    /// A Taffy input for this node or one of its descendants has changed. Taffy's cache should be cleared for this node.
    LayoutDirty,
    /// This node or a descendant needs its `ComputedNode`, `UiGlobalTransform`, or `CalculatedClip` recomputed, but Taffy inputs were unchanged.
    GeometryDirty,
}

impl SubtreeState {
    const fn needs_relayout(self) -> bool {
        matches!(self, Self::LayoutDirty)
    }

    const fn needs_geometry(self) -> bool {
        matches!(self, Self::LayoutDirty | Self::GeometryDirty)
    }

    /// Combines the updates required by two states.
    /// `LayoutDirty` requires both layout and geometry updates, so it includes `GeometryDirty`.
    /// The combined state is only `Clean` if both source states are clean.
    #[must_use]
    const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::LayoutDirty, _) | (_, Self::LayoutDirty) => Self::LayoutDirty,
            (Self::GeometryDirty, _) | (_, Self::GeometryDirty) => Self::GeometryDirty,
            (Self::Clean, Self::Clean) => Self::Clean,
        }
    }
}

/// Synchronizes the cached hierarchy stored in the `ComputedLayout` components and resolves its dirty state.
fn sync_layout_tree(
    root: Entity,
    entity: Entity,
    ui_children: &Query<(Option<&Children>, Has<GhostNode>, Ref<UiTreeDirty>), With<Node>>,
    node_query: &Query<
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
    computed_layout_query: &mut Query<&mut ComputedLayout>,
    fixed_node_changes: &EntityHashSet,
    child_stack: &mut Vec<NodeId>,
    needs_full_walk: bool,
    ghost_stack: &mut Vec<Entity>,
) -> Option<SubtreeState> {
    let Ok((
        style,
        content_size,
        has_fixed_node,
        transform,
        scroll_position,
        outline,
        layout_config,
        ignore_scroll,
        has_override_clip,
        tree_changed,
    )) = node_query.get(entity)
    else {
        // `entity` is not a UI node.
        return None;
    };

    // Nested `FixedNode`s are laid out separately as roots.
    // `GhostNode`s are already flattened, an entity with both `FixedNode` and `GhostNode` can't be reached here.
    if has_fixed_node && entity != root {
        return None;
    }

    // Nothing in this subtree changed, so its cached children, cache and flags are all
    // still valid. Reported clean, and left for the next full walk to mark as reached.
    if !needs_full_walk && !tree_changed.is_changed() {
        return Some(SubtreeState::Clean);
    }

    let mut subtree_state = SubtreeState::Clean;
    let start = child_stack.len();
    let dirty_ghost = collect_ui_children(entity, ui_children, child_stack, ghost_stack);
    let end = child_stack.len();
    let mut child_count = 0;
    for child_index in start..end {
        let child_node = child_stack[child_index];
        if let Some(built_subtree_state) = sync_layout_tree(
            root,
            entity_from_node_id(child_node).expect("Viewport's NodeId found in child stack."),
            ui_children,
            node_query,
            computed_layout_query,
            fixed_node_changes,
            child_stack,
            needs_full_walk,
            ghost_stack,
        ) {
            child_stack[start + child_count] = child_node;
            child_count += 1;
            subtree_state = subtree_state.merge(built_subtree_state);
        }
    }
    let end = start + child_count;
    child_stack.truncate(end);

    let Ok(mut computed_layout) = computed_layout_query.get_mut(entity) else {
        return None;
    };
    let was_root = computed_layout.is_layout_root;
    computed_layout.is_layout_root = entity == root;
    let new_children = &child_stack[start..end];
    let children_changed = computed_layout.ui_children != new_children;
    if children_changed {
        computed_layout.ui_children.clear();
        computed_layout.hoisted_children.clear();
        computed_layout.ui_children.extend_from_slice(new_children);
    }

    child_stack.truncate(start);

    let own_dirty = style.is_changed()
        || children_changed
        || was_root != computed_layout.is_layout_root
        || content_size.is_changed()
        || fixed_node_changes.contains(&entity)
        || !computed_layout.has_layout();

    if own_dirty {
        subtree_state = subtree_state.merge(SubtreeState::LayoutDirty);
    }

    computed_layout.reached_in_full_walk |= needs_full_walk;
    computed_layout.layout_dirty = false;

    let outline_changed = (computed_layout.has_outline != outline.is_some())
        || outline.is_some_and(|outline| outline.is_changed());
    let layout_config_changed = (computed_layout.has_layout_config != layout_config.is_some())
        || layout_config.is_some_and(|layout_config| layout_config.is_changed());
    let ignore_scroll_changed = (computed_layout.has_ignore_scroll != ignore_scroll.is_some())
        || ignore_scroll.is_some_and(|ignore_scroll| ignore_scroll.is_changed());
    let override_clip_changed = computed_layout.has_override_clip != has_override_clip;

    computed_layout.self_dirty = own_dirty
        || transform.is_changed()
        || scroll_position.is_changed()
        || outline_changed
        || layout_config_changed
        || ignore_scroll_changed
        || override_clip_changed;

    computed_layout.has_ignore_scroll = ignore_scroll.is_some();
    computed_layout.has_layout_config = layout_config.is_some();
    computed_layout.has_outline = outline.is_some();
    computed_layout.has_override_clip = has_override_clip;

    if computed_layout.self_dirty || dirty_ghost {
        subtree_state = subtree_state.merge(SubtreeState::GeometryDirty);
    }
    computed_layout.subtree_dirty = subtree_state.needs_geometry();

    if subtree_state.needs_relayout() {
        computed_layout.cache.clear();
    }

    Some(subtree_state)
}

struct UiLayoutTree<'a, 'w, 's> {
    style_query: &'a Query<'w, 's, &'static TaffyStyle>,
    computed_layout_query: &'a mut Query<'w, 's, &'static mut ComputedLayout>,
    viewport_layout: ComputedLayout,
    measure_function: &'a mut dyn FnMut(
        taffy::Size<Option<f32>>,
        taffy::Size<AvailableSpace>,
        Entity,
        &Style,
    ) -> taffy::Size<f32>,
    layout_changed: bool,
}

impl<'a> UiLayoutTree<'a, '_, '_> {
    fn layout(&self, node_id: NodeId) -> &ComputedLayout {
        entity_from_node_id(node_id).map_or(&self.viewport_layout, |entity| {
            self.computed_layout_query
                .get(entity)
                .expect("Missing ComputedLayout on UI node.")
        })
    }

    fn layout_mut(&mut self, node_id: NodeId) -> &mut ComputedLayout {
        entity_from_node_id(node_id).map_or(&mut self.viewport_layout, |entity| {
            self.computed_layout_query
                .get_mut(entity)
                .expect("Missing ComputedLayout on UI node.")
                .into_inner()
        })
    }

    fn style(&self, node_id: NodeId) -> &'a Style {
        entity_from_node_id(node_id).map_or(&VIEWPORT_NODE_TAFFY_STYLE, |entity| {
            self.style_query
                .get(entity)
                .expect("Missing TaffyStyle on UI node.")
        })
    }

    fn children(&self, node_id: NodeId) -> &[NodeId] {
        &self.layout(node_id).ui_children
    }

    #[inline(always)]
    fn compute_child_layout_with_context(
        &mut self,
        node_id: NodeId,
        inputs: LayoutInput,
        block_ctx: Option<&mut taffy::BlockContext<'_>>,
    ) -> LayoutOutput {
        if inputs.run_mode == RunMode::PerformHiddenLayout {
            return compute_hidden_layout(self, node_id);
        }

        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let has_children = 0 < tree.child_count(node_id);
            let style = tree.style(node_id);

            let mut output = match (style.display, has_children) {
                (Display::None, _) => compute_hidden_layout(tree, node_id),
                (Display::Block, true) => compute_block_layout(tree, node_id, inputs, block_ctx),
                (Display::Flex, true) => compute_flexbox_layout(tree, node_id, inputs),
                (Display::Grid, true) => compute_grid_layout(tree, node_id, inputs),
                // There's no matching `FlowRoot` variant for `bevy_ui::Display`, so this is unreachable.
                (Display::FlowRoot, _) => unreachable!(),
                (_, false) => compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.0,
                    |known_dimensions, available_space| {
                        (tree.measure_function)(
                            known_dimensions,
                            available_space,
                            entity_from_node_id(node_id)
                                .expect("Viewport Node should not be a child"),
                            style,
                        )
                    },
                ),
            };

            if inputs.run_mode == RunMode::PerformLayout {
                compute_oof_layout(tree, node_id, &mut output);
            }

            output
        })
    }
}

impl TraversePartialTree for UiLayoutTree<'_, '_, '_> {
    type ChildIter<'a>
        = core::iter::Copied<core::slice::Iter<'a, NodeId>>
    where
        Self: 'a;

    fn child_ids(&self, parent_node_id: NodeId) -> Self::ChildIter<'_> {
        self.children(parent_node_id).iter().copied()
    }

    fn child_count(&self, parent_node_id: NodeId) -> usize {
        self.children(parent_node_id).len()
    }

    fn get_child_id(&self, parent_node_id: NodeId, child_index: usize) -> NodeId {
        self.children(parent_node_id)[child_index]
    }
}

impl TraverseTree for UiLayoutTree<'_, '_, '_> {}

impl<'tree, 'w, 's> LayoutPartialTree for UiLayoutTree<'tree, 'w, 's> {
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        self.style(node_id)
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.layout_changed |=
            self.layout_mut(node_id).set_unrounded(*layout) && node_id != VIEWPORT_NODE_ID;
    }

    fn compute_child_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        self.compute_child_layout_with_context(node_id, inputs, None)
    }
}

impl CacheTree for UiLayoutTree<'_, '_, '_> {
    fn cache_get(&mut self, node_id: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.layout_mut(node_id).cache.get(input)
    }

    fn cache_store(&mut self, node_id: NodeId, input: &LayoutInput, layout_output: LayoutOutput) {
        self.layout_mut(node_id).cache.store(input, layout_output);
    }

    fn cache_clear(&mut self, node_id: NodeId) {
        self.layout_mut(node_id).cache.clear();
    }
}

impl<'tree, 'w, 's> LayoutBlockContainer for UiLayoutTree<'tree, 'w, 's> {
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        self.style(node_id)
    }

    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        self.style(child_node_id)
    }

    #[inline(always)]
    fn compute_block_child_layout(
        &mut self,
        node_id: NodeId,
        inputs: LayoutInput,
        block_ctx: Option<&mut taffy::BlockContext<'_>>,
    ) -> LayoutOutput {
        self.compute_child_layout_with_context(node_id, inputs, block_ctx)
    }
}

impl<'tree, 'w, 's> LayoutFlexboxContainer for UiLayoutTree<'tree, 'w, 's> {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.style(node_id)
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.style(child_node_id)
    }
}

impl<'tree, 'w, 's> LayoutGridContainer for UiLayoutTree<'tree, 'w, 's> {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: NodeId) -> Self::GridContainerStyle<'_> {
        self.style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: NodeId) -> Self::GridItemStyle<'_> {
        self.style(child_node_id)
    }
}

impl RoundTree for UiLayoutTree<'_, '_, '_> {
    fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
        self.layout(node_id)
            .unrounded
            .expect("missing unrounded layout")
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        if node_id == VIEWPORT_NODE_ID {
            // viewport nodes only use the unrounded layout
            return;
        }

        self.layout_mut(node_id).set_rounded(*layout);
    }

    fn is_out_of_flow(&self, node_id: NodeId) -> bool {
        self.style(node_id).position.is_out_of_flow()
    }

    fn hoisted_child_count(&self, node_id: NodeId) -> usize {
        self.layout(node_id).hoisted_children.len()
    }

    fn get_hoisted_child_id(&self, node_id: NodeId, index: usize) -> NodeId {
        self.layout(node_id).hoisted_children[index]
    }
}

impl<'tree, 'w, 's> LayoutContainingBlock for UiLayoutTree<'tree, 'w, 's> {
    type OofItemStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_oof_item_style(&self, node_id: NodeId) -> Self::OofItemStyle<'_> {
        self.style(node_id)
    }

    fn clear_hoisted_children(&mut self, node_id: NodeId) {
        self.layout_mut(node_id).hoisted_children.clear();
    }

    fn add_hoisted_children(&mut self, node_id: NodeId, children: &[NodeId]) {
        self.layout_mut(node_id)
            .hoisted_children
            .extend_from_slice(children);
    }
}

pub fn get_text_buffer<'a>(
    needs_buffer: bool,
    ctx: &NodeMeasure,
    query: &'a mut Query<&mut ComputedTextBlock>,
) -> Option<&'a mut ComputedTextBlock> {
    // We avoid a query lookup whenever the buffer is not required.
    if !needs_buffer {
        return None;
    }
    let NodeMeasure::Text(crate::widget::TextMeasure { info }) = ctx else {
        return None;
    };
    let Ok(computed) = query.get_mut(info.entity) else {
        return None;
    };
    Some(computed.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_layout_returns_none() {
        let computed_layout = ComputedLayout::default();
        assert!(computed_layout.get_layout(true).is_none());
    }
}
