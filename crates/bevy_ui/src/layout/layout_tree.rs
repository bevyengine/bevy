use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    entity::Entity,
    query::Has,
    system::Query,
    world::Ref,
};
use bevy_math::{UVec2, Vec2};
use bevy_text::{ComputedTextBlock, FontCx, RemSize};
use taffy::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_leaf_layout, compute_root_layout, round_layout,
    style::{AvailableSpace, Display, Style},
    Cache, CacheTree, Layout, LayoutBlockContainer, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, RoundTree, RunMode, TraversePartialTree,
    TraverseTree,
};

use crate::{
    experimental::UiChildren, style::TaffyStyle, ContentSize, FixedNode, LayoutError, Measure,
    MeasureArgs, NodeMeasure,
};

pub static VIEWPORT_NODE_TAFFY_STYLE: TaffyStyle = TaffyStyle(Style {
    dummy: core::marker::PhantomData,
    display: Display::Grid,
    item_is_table: false,
    item_is_replaced: false,
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
    align_items: Some(taffy::AlignItems::START),
    align_self: None,
    justify_items: Some(taffy::JustifyItems::START),
    justify_self: None,
    align_content: None,
    justify_content: None,
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

const fn entity_node_id(entity: Entity) -> NodeId {
    NodeId::new(entity.to_bits())
}

/// `entity.to_bits()` can't be zero, so we use a `NodeId` of zero to represent viewport nodes.
pub const VIEWPORT_NODE_ID: NodeId = NodeId::new(0u64);

fn node_id_entity(node_id: NodeId) -> Entity {
    Entity::try_from_bits(u64::from(node_id)).expect("Tried to get an entity for a viewport node.")
}

/// Cached and computed layout state for a UI node.
#[derive(Component, Debug, Clone, Default)]
pub struct ComputedLayout {
    /// unrounded layout
    unrounded: Option<Layout>,
    /// rounded layout
    rounded: Option<Layout>,
    /// cached sizing results
    cache: Cache,
    /// was visited during layout
    visited: bool,
    /// children
    children: Vec<NodeId>,
}

impl ComputedLayout {
    /// Clear all state
    pub fn clear(&mut self) {
        self.unrounded = None;
        self.rounded = None;
        self.cache.clear();
        self.children.clear();
        self.visited = false;
    }

    /// Returns true if both rounded and unrounded layouts are present
    pub fn has_layout(&self) -> bool {
        self.unrounded.is_some() && self.rounded.is_some()
    }

    /// Set rounded layout
    pub fn set_rounded(&mut self, layout: Layout) {
        self.rounded = Some(layout);
    }

    /// Returns `true` if the layout changed since the last update
    pub fn set_unrounded(&mut self, layout: Layout) -> bool {
        if self.unrounded == Some(layout) {
            return false;
        }
        self.unrounded = Some(layout);
        true
    }

    /// Set visited state
    pub fn set_visited(&mut self, visited: bool) {
        self.visited = visited;
    }

    /// Returns true if visited
    pub fn visited(&self) -> bool {
        self.visited
    }

    /// Get the layout geometry and size
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
}

/// Compute and store layout results for one UI root entity.
pub(crate) fn compute_layout(
    ui_root_entity: Entity,
    render_target_resolution: UVec2,
    ui_children: &UiChildren,
    node_query: &Query<(Ref<TaffyStyle>, Ref<ContentSize>, Has<FixedNode>)>,
    style_query: &Query<&TaffyStyle>,
    computed_layout_query: &mut Query<&mut ComputedLayout>,
    fixed_node_changes: &[Entity],
    buffer_query: &mut Query<&mut ComputedTextBlock>,
    font_system: &mut FontCx,
    rem_size: RemSize,
    rem_size_changed: bool,
) -> Result<(), LayoutError> {
    let Some(_) = build_runtime_layout_tree(
        ui_root_entity,
        ui_root_entity,
        ui_children,
        node_query,
        computed_layout_query,
        fixed_node_changes,
        rem_size,
    ) else {
        return Err(LayoutError::InvalidHierarchy);
    };
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
            viewport_layout: ViewportLayoutState::default(),
            viewport_children: [root_node_id],
            measure_function: &mut measure_function,
            layout_changed: false,
        };

        compute_root_layout(&mut tree, VIEWPORT_NODE_ID, available_space);
        if tree.layout_changed {
            round_layout(&mut tree, VIEWPORT_NODE_ID);
        }
    };

    Ok(())
}

fn build_runtime_layout_tree<'a>(
    root: Entity,
    entity: Entity,
    ui_children: &UiChildren,
    node_query: &'a Query<(Ref<TaffyStyle>, Ref<ContentSize>, Has<FixedNode>)>,
    computed_layout_query: &mut Query<&mut ComputedLayout>,
    fixed_node_changes: &[Entity],
    rem_size: RemSize,
) -> Option<bool> {
    let Ok((style, content_size, has_fixed_node)) = node_query.get(entity) else {
        return None;
    };

    if has_fixed_node && entity != root {
        return None;
    }

    let mut new_children = Vec::new();
    let mut subtree_dirty = false;
    for child in ui_children.iter_ui_children(entity) {
        if let Some(built_child_dirty) = build_runtime_layout_tree(
            root,
            child,
            ui_children,
            node_query,
            computed_layout_query,
            fixed_node_changes,
            rem_size,
        ) {
            new_children.push(entity_node_id(child));
            subtree_dirty |= built_child_dirty;
        }
    }

    let Ok(mut computed_layout) = computed_layout_query.get_mut(entity) else {
        return None;
    };
    let computed_layout = computed_layout.bypass_change_detection();
    let children_changed = computed_layout.children != new_children;
    if children_changed {
        computed_layout.children.clear();
        computed_layout.children.extend_from_slice(&new_children);
    }

    let own_dirty = style.is_changed()
        || children_changed
        || content_size.is_changed()
        || fixed_node_changes.contains(&entity)
        || !computed_layout.has_layout();
    subtree_dirty |= own_dirty;

    computed_layout.visited = true;
    if subtree_dirty {
        computed_layout.cache.clear();
    }

    Some(subtree_dirty)
}

#[derive(Default)]
struct ViewportLayoutState {
    cache: Cache,
    unrounded: Layout,
}

struct UiLayoutTree<'a, 'w, 's, 'u, 't, 'style, 'layout> {
    //nodes: HashMap<NodeId, NodeStyle<'node>>,
    style_query: &'a Query<'w, 's, &'style TaffyStyle>,
    computed_layout_query: &'a mut Query<'u, 't, &'layout mut ComputedLayout>,
    viewport_layout: ViewportLayoutState,
    viewport_children: [NodeId; 1],
    measure_function: &'a mut dyn FnMut(
        taffy::Size<Option<f32>>,
        taffy::Size<AvailableSpace>,
        Entity,
        &Style,
    ) -> taffy::Size<f32>,
    layout_changed: bool,
}

impl UiLayoutTree<'_, '_, '_, '_, '_, '_, '_> {
    fn children(&self, node_id: NodeId) -> &[NodeId] {
        if node_id == VIEWPORT_NODE_ID {
            return &self.viewport_children;
        }
        &self
            .computed_layout_query
            .get(node_id_entity(node_id))
            .expect("missing computed layout")
            .children
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
            let has_children = tree.child_count(node_id) > 0;

            match (tree.get_core_container_style(node_id).display, has_children) {
                (Display::None, _) => compute_hidden_layout(tree, node_id),
                (Display::Block, true) => compute_block_layout(tree, node_id, inputs, block_ctx),
                (Display::Flex, true) => compute_flexbox_layout(tree, node_id, inputs),
                (Display::Grid, true) => compute_grid_layout(tree, node_id, inputs),
                // There's no matching `FlowRoot` variant for `bevy_ui::Display`, so this is unreachable.
                (Display::FlowRoot, _) => unreachable!(),
                (_, false) => {
                    let style = &tree
                        .style_query
                        .get(node_id_entity(node_id))
                        .expect("missing layout style")
                        .0;
                    compute_leaf_layout(
                        inputs,
                        style,
                        |_, _| 0.0,
                        |known_dimensions, available_space| {
                            let entity = node_id_entity(node_id);
                            (tree.measure_function)(
                                known_dimensions,
                                available_space,
                                entity,
                                style,
                            )
                        },
                    )
                }
            }
        })
    }
}

impl TraversePartialTree for UiLayoutTree<'_, '_, '_, '_, '_, '_, '_> {
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

impl TraverseTree for UiLayoutTree<'_, '_, '_, '_, '_, '_, '_> {}

impl<'tree, 'w, 's, 'u, 't, 'style, 'layout> LayoutPartialTree
    for UiLayoutTree<'tree, 'w, 's, 'u, 't, 'style, 'layout>
{
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        if node_id == VIEWPORT_NODE_ID {
            &VIEWPORT_NODE_TAFFY_STYLE
        } else {
            &self
                .style_query
                .get(node_id_entity(node_id))
                .expect("missing layout node")
                .0
        }
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        if node_id == VIEWPORT_NODE_ID {
            // Viewport layout is only ever used unrounded.
            self.viewport_layout.unrounded = *layout;
            return;
        }

        let entity = node_id_entity(node_id);

        self.layout_changed |= self
            .computed_layout_query
            .get_mut(entity)
            .expect("missing computed layout")
            .bypass_change_detection()
            .set_unrounded(*layout);
    }

    fn compute_child_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        self.compute_child_layout_with_context(node_id, inputs, None)
    }
}

impl CacheTree for UiLayoutTree<'_, '_, '_, '_, '_, '_, '_> {
    fn cache_get(&mut self, node_id: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        if node_id == VIEWPORT_NODE_ID {
            return self.viewport_layout.cache.get(input);
        }

        let entity = node_id_entity(node_id);
        self.computed_layout_query
            .get_mut(entity)
            .expect("missing computed layout")
            .bypass_change_detection()
            .cache
            .get(input)
    }

    fn cache_store(&mut self, node_id: NodeId, input: &LayoutInput, layout_output: LayoutOutput) {
        if node_id == VIEWPORT_NODE_ID {
            self.viewport_layout.cache.store(input, layout_output);
            return;
        }

        let entity = node_id_entity(node_id);
        self.computed_layout_query
            .get_mut(entity)
            .expect("missing computed layout")
            .bypass_change_detection()
            .cache
            .store(input, layout_output);
    }

    fn cache_clear(&mut self, node_id: NodeId) {
        if node_id == VIEWPORT_NODE_ID {
            self.viewport_layout.cache.clear();
            return;
        }

        let entity = node_id_entity(node_id);
        self.computed_layout_query
            .get_mut(entity)
            .expect("missing computed layout")
            .bypass_change_detection()
            .cache
            .clear();
    }
}

impl<'tree, 'w, 's, 'u, 't, 'style, 'layout> LayoutBlockContainer
    for UiLayoutTree<'tree, 'w, 's, 'u, 't, 'style, 'layout>
{
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        self.get_core_container_style(child_node_id)
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

impl<'tree, 'w, 's, 'u, 't, 'style, 'layout> LayoutFlexboxContainer
    for UiLayoutTree<'tree, 'w, 's, 'u, 't, 'style, 'layout>
{
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl<'tree, 'w, 's, 'u, 't, 'style, 'layout> LayoutGridContainer
    for UiLayoutTree<'tree, 'w, 's, 'u, 't, 'style, 'layout>
{
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: NodeId) -> Self::GridContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: NodeId) -> Self::GridItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl RoundTree for UiLayoutTree<'_, '_, '_, '_, '_, '_, '_> {
    fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
        if node_id == VIEWPORT_NODE_ID {
            return self.viewport_layout.unrounded;
        }

        let entity = node_id_entity(node_id);
        self.computed_layout_query
            .get(entity)
            .expect("missing computed layout")
            .unrounded
            .expect("missing unrounded layout")
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        if node_id == VIEWPORT_NODE_ID {
            // viewport nodes only use the unrounded layout
            return;
        }

        let entity = node_id_entity(node_id);
        self.computed_layout_query
            .get_mut(entity)
            .expect("missing computed layout")
            .bypass_change_detection()
            .set_rounded(*layout);
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
