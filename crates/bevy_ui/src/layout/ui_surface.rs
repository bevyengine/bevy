use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    entity::Entity,
    query::Has,
    system::Query,
    world::Ref,
};
use bevy_math::{UVec2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_text::{ComputedTextBlock, EmSize, FontCx, RemSize};
use taffy::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_leaf_layout, compute_root_layout, round_layout,
    style::{AvailableSpace, Display, Style},
    Cache, CacheTree, Layout, LayoutBlockContainer, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, RoundTree, RunMode, TraversePartialTree,
    TraverseTree,
};

use crate::{
    experimental::UiChildren,
    layout::style::{NodeStyle, VIEWPORT_NODE},
    ComputedUiRenderTargetInfo, ContentSize, FixedNode, LayoutContext, LayoutError, Measure,
    MeasureArgs, Node, NodeMeasure,
};

const fn entity_node_id(entity: Entity) -> NodeId {
    NodeId::new(entity.to_bits())
}

/// `entity.to_bits()` can't be zero, so we use a `NodeId` of zero to represent viewport nodes.
pub const VIEWPORT_NODE_ID: NodeId = NodeId::new(0u64);

fn node_id_entity(node_id: NodeId) -> Entity {
    Entity::try_from_bits(u64::from(node_id)).expect("missing layout entity")
}

#[derive(Component, Debug, Clone, Default)]
#[doc(hidden)]
pub struct ComputedLayout {
    unrounded: Option<Layout>,
    rounded: Option<Layout>,
    cache: Cache,
    visited: bool,
    // children from previous frame
    children: Vec<NodeId>,
}

impl ComputedLayout {
    pub(crate) fn clear(&mut self) {
        self.unrounded = None;
        self.rounded = None;
    }

    pub(crate) fn prepare_for_layout(&mut self) {
        self.visited = false;
    }

    /// Used to clear stale layout state if a node is unreachable
    pub(crate) fn clear_if_unreachable(&mut self) {
        if !self.visited {
            self.clear();
            self.cache.clear();
            self.children.clear();
        }
    }

    fn mark_visited(&mut self) {
        self.visited = true;
    }

    fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn has_layout(&self) -> bool {
        self.unrounded.is_some() && self.rounded.is_some()
    }

    /// Returns `true` if the layout changed since the last update
    fn set_unrounded(&mut self, layout: Layout) -> bool {
        if self.unrounded == Some(layout) {
            return false;
        }
        self.unrounded = Some(layout);
        true
    }

    fn set_rounded(&mut self, layout: Layout) {
        self.rounded = Some(layout);
    }

    pub fn get(&self, use_rounding: bool) -> Option<(Layout, Vec2)> {
        let unrounded = self.unrounded?;
        let selected_layout = if use_rounding {
            self.rounded?
        } else {
            unrounded
        };
        let unrounded_size = Vec2::new(unrounded.size.width, unrounded.size.height);

        Some((selected_layout, unrounded_size))
    }

    // Replace the previous children list, returning true if different.
    fn set_children(&mut self, children: &[NodeId]) -> bool {
        if self.children == children {
            return false;
        }

        self.children.clear();
        self.children.extend_from_slice(children);
        true
    }
}

/// Compute and store layout results for one UI root entity.
pub(crate) fn compute_layout(
    ui_root_entity: Entity,
    render_target_resolution: UVec2,
    ui_children: &UiChildren,
    node_query: &Query<(
        Ref<Node>,
        Ref<ComputedUiRenderTargetInfo>,
        Ref<EmSize>,
        Ref<ContentSize>,
        Has<FixedNode>,
    )>,
    computed_layout_query: &mut Query<&mut ComputedLayout>,
    fixed_node_changes: &[Entity],
    buffer_query: &mut Query<&mut ComputedTextBlock>,
    font_system: &mut FontCx,
    rem_size: RemSize,
    rem_size_changed: bool,
) -> Result<(), LayoutError> {
    let mut runtime_nodes = HashMap::default();
    let Some(_) = build_runtime_layout_tree(
        ui_root_entity,
        ui_root_entity,
        ui_children,
        node_query,
        computed_layout_query,
        fixed_node_changes,
        &mut runtime_nodes,
        rem_size,
        rem_size_changed,
    ) else {
        return Err(LayoutError::InvalidHierarchy);
    };
    let root_node_id = entity_node_id(ui_root_entity);

    runtime_nodes.insert(
        VIEWPORT_NODE_ID,
        NodeStyle {
            node: &VIEWPORT_NODE,
            context: LayoutContext::default(),
        },
    );

    let available_space = taffy::Size {
        width: AvailableSpace::Definite(render_target_resolution.x as f32),
        height: AvailableSpace::Definite(render_target_resolution.y as f32),
    };

    {
        let mut measure_function = |known_dimensions: taffy::Size<Option<f32>>,
                                    available_space: taffy::Size<AvailableSpace>,
                                    entity: Entity,
                                    style: &Style| {
            let Ok((_, _, _, content_size, _)) = node_query.get(entity) else {
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
            nodes: runtime_nodes,
            computed_layout_query,
            viewport_layout: LayoutState::default(),
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
    node_query: &'a Query<(
        Ref<Node>,
        Ref<ComputedUiRenderTargetInfo>,
        Ref<EmSize>,
        Ref<ContentSize>,
        Has<FixedNode>,
    )>,
    computed_layout_query: &mut Query<&mut ComputedLayout>,
    fixed_node_changes: &[Entity],
    runtime_nodes: &mut HashMap<NodeId, NodeStyle<'a>>,
    rem_size: RemSize,
    rem_size_changed: bool,
) -> Option<bool> {
    let Ok((node, computed_target, em_size, content_size, has_fixed_node)) = node_query.get(entity)
    else {
        return None;
    };

    if has_fixed_node && entity != root {
        return None;
    }

    let mut child_ids = Vec::new();
    let mut subtree_dirty = false;
    for child in ui_children.iter_ui_children(entity) {
        if let Some(built_child_dirty) = build_runtime_layout_tree(
            root,
            child,
            ui_children,
            node_query,
            computed_layout_query,
            fixed_node_changes,
            runtime_nodes,
            rem_size,
            rem_size_changed,
        ) {
            child_ids.push(entity_node_id(child));
            subtree_dirty |= built_child_dirty;
        }
    }

    let Ok(mut computed_layout) = computed_layout_query.get_mut(entity) else {
        return None;
    };
    let computed_layout = computed_layout.bypass_change_detection();

    let node_id = entity_node_id(entity);
    let children_changed = computed_layout.set_children(&child_ids);
    let own_dirty = node.is_changed()
        || em_size.is_changed()
        || rem_size_changed
        || children_changed
        || computed_target.is_changed()
        || content_size.is_changed()
        || fixed_node_changes.contains(&entity)
        || !computed_layout.has_layout();
    subtree_dirty |= own_dirty;

    let layout_context = LayoutContext::new(
        computed_target.scale_factor(),
        computed_target.physical_size().as_vec2(),
        *em_size,
        rem_size,
    );
    let node = node.into_inner();

    computed_layout.mark_visited();
    if subtree_dirty {
        computed_layout.clear_cache();
    }

    runtime_nodes.insert(node_id, NodeStyle::from_node(node, layout_context));

    Some(subtree_dirty)
}

#[derive(Default)]
struct LayoutState {
    cache: Cache,
    unrounded: Layout,
    rounded: Layout,
}

struct UiLayoutTree<'a, 'w, 's, 'layout, 'node> {
    nodes: HashMap<NodeId, NodeStyle<'node>>,
    computed_layout_query: &'a mut Query<'w, 's, &'layout mut ComputedLayout>,
    viewport_layout: LayoutState,
    viewport_children: [NodeId; 1],
    measure_function: &'a mut dyn FnMut(
        taffy::Size<Option<f32>>,
        taffy::Size<AvailableSpace>,
        Entity,
        &Style,
    ) -> taffy::Size<f32>,
    layout_changed: bool,
}

impl UiLayoutTree<'_, '_, '_, '_, '_> {
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
}

impl TraversePartialTree for UiLayoutTree<'_, '_, '_, '_, '_> {
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

impl TraverseTree for UiLayoutTree<'_, '_, '_, '_, '_> {}

impl<'tree, 'w, 's, 'layout, 'node> LayoutPartialTree
    for UiLayoutTree<'tree, 'w, 's, 'layout, 'node>
{
    type CoreContainerStyle<'a>
        = &'a NodeStyle<'node>
    where
        Self: 'a;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        self.nodes.get(&node_id).expect("missing layout node")
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
        if inputs.run_mode == RunMode::PerformHiddenLayout {
            return compute_hidden_layout(self, node_id);
        }

        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let style = tree
                .nodes
                .get(&node_id)
                .expect("missing layout node")
                .clone();
            let has_children = tree.child_count(node_id) > 0;

            match (style.display(), has_children) {
                (Display::None, _) => compute_hidden_layout(tree, node_id),
                (Display::Block, true) => compute_block_layout(tree, node_id, inputs, None),
                (Display::Flex, true) => compute_flexbox_layout(tree, node_id, inputs),
                (Display::Grid, true) => compute_grid_layout(tree, node_id, inputs),
                (Display::FlowRoot, true) => compute_grid_layout(tree, node_id, inputs),
                (_, false) => compute_leaf_layout(
                    inputs,
                    &style,
                    |_, _| 0.0,
                    |known_dimensions, available_space| {
                        let taffy_style = style.to_taffy_style();
                        let entity = node_id_entity(node_id);
                        (tree.measure_function)(
                            known_dimensions,
                            available_space,
                            entity,
                            &taffy_style,
                        )
                    },
                ),
            }
        })
    }
}

impl CacheTree for UiLayoutTree<'_, '_, '_, '_, '_> {
    fn cache_get(&self, node_id: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        if node_id == VIEWPORT_NODE_ID {
            return self.viewport_layout.cache.get(input);
        }

        let entity = node_id_entity(node_id);
        self.computed_layout_query
            .get(entity)
            .expect("missing computed layout")
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

impl<'tree, 'w, 's, 'layout, 'node> LayoutBlockContainer
    for UiLayoutTree<'tree, 'w, 's, 'layout, 'node>
{
    type BlockContainerStyle<'a>
        = &'a NodeStyle<'node>
    where
        Self: 'a;

    type BlockItemStyle<'a>
        = &'a NodeStyle<'node>
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl<'tree, 'w, 's, 'layout, 'node> LayoutFlexboxContainer
    for UiLayoutTree<'tree, 'w, 's, 'layout, 'node>
{
    type FlexboxContainerStyle<'a>
        = &'a NodeStyle<'node>
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = &'a NodeStyle<'node>
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl<'tree, 'w, 's, 'layout, 'node> LayoutGridContainer
    for UiLayoutTree<'tree, 'w, 's, 'layout, 'node>
{
    type GridContainerStyle<'a>
        = &'a NodeStyle<'node>
    where
        Self: 'a;

    type GridItemStyle<'a>
        = &'a NodeStyle<'node>
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: NodeId) -> Self::GridContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: NodeId) -> Self::GridItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl RoundTree for UiLayoutTree<'_, '_, '_, '_, '_> {
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
            self.viewport_layout.rounded = *layout;
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
        assert!(computed_layout.get(true).is_none());
    }
}
