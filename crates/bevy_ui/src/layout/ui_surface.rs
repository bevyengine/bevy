use core::fmt;

use bevy_ecs::{
    change_detection::DetectChangesMut, component::Component, entity::Entity, prelude::Resource,
    system::Query,
};
use bevy_math::{UVec2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_text::{ComputedTextBlock, FontCx};
use taffy::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_leaf_layout, compute_root_layout, round_layout,
    style::{AvailableSpace, Display, Style},
    Cache, CacheTree, Layout, LayoutBlockContainer, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, RoundTree, RunMode, TraversePartialTree,
    TraverseTree,
};

use crate::{
    experimental::UiChildren, layout::style::CoreNode, ComputedUiRenderTargetInfo, ContentSize,
    LayoutContext, LayoutError, Measure, MeasureArgs, Node, NodeMeasure,
};

fn entity_node_id(entity: Entity) -> NodeId {
    NodeId::from(entity.to_bits())
}

fn viewport_node_id() -> NodeId {
    NodeId::from(u64::MAX)
}

#[derive(Component, Debug, Copy, Clone, Default)]
#[doc(hidden)]
pub struct ComputedLayout {
    unrounded: Option<Layout>,
    rounded: Option<Layout>,
}

impl ComputedLayout {
    pub(crate) fn clear(&mut self) {
        self.unrounded = None;
        self.rounded = None;
    }

    fn set(&mut self, unrounded: Layout, rounded: Layout) {
        self.unrounded = Some(unrounded);
        self.rounded = Some(rounded);
    }

    pub(crate) fn get(&self, use_rounding: bool) -> Option<(Layout, Vec2)> {
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

#[derive(Resource, Default)]
pub struct UiSurface;

impl fmt::Debug for UiSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiSurface").finish()
    }
}

impl UiSurface {
    /// Compute and store layout results for one UI root entity.
    pub(crate) fn compute_layout(
        &mut self,
        ui_root_entity: Entity,
        render_target_resolution: UVec2,
        ui_children: &UiChildren,
        node_query: &Query<(Entity, &Node, &ComputedUiRenderTargetInfo)>,
        layout_query: &mut Query<(Entity, &mut ContentSize, &mut ComputedLayout)>,
        buffer_query: &mut Query<&mut ComputedTextBlock>,
        font_system: &mut FontCx,
    ) -> Result<(), LayoutError> {
        let mut runtime_nodes = HashMap::default();
        if !build_runtime_layout_tree(
            ui_root_entity,
            ui_children,
            node_query,
            layout_query,
            &mut runtime_nodes,
        )? {
            return Err(LayoutError::InvalidHierarchy);
        }

        let root_node_id = entity_node_id(ui_root_entity);
        runtime_nodes.insert(
            viewport_node_id(),
            RuntimeLayoutNode::viewport(root_node_id),
        );

        let available_space = taffy::Size {
            width: AvailableSpace::Definite(render_target_resolution.x as f32),
            height: AvailableSpace::Definite(render_target_resolution.y as f32),
        };

        let runtime_nodes = {
            let mut measure_function = |known_dimensions: taffy::Size<Option<f32>>,
                                        available_space: taffy::Size<AvailableSpace>,
                                        measure: &mut NodeMeasure,
                                        style: &Style| {
                let buffer = get_text_buffer(
                    crate::widget::TextMeasure::needs_buffer(
                        known_dimensions.height,
                        available_space.width,
                    ),
                    measure,
                    buffer_query,
                );
                let size = measure.measure(MeasureArgs {
                    known_width: known_dimensions.width,
                    known_height: known_dimensions.height,
                    available_width: available_space.width,
                    available_height: available_space.height,
                    font_system,
                    buffer,
                    style,
                });
                taffy::Size {
                    width: size.x,
                    height: size.y,
                }
            };

            let mut tree = EcsLayoutTree {
                nodes: runtime_nodes,
                measure_function: &mut measure_function,
            };

            compute_root_layout(&mut tree, viewport_node_id(), available_space);
            round_layout(&mut tree, viewport_node_id());
            tree.nodes
        };

        for runtime_node in runtime_nodes.into_values() {
            let Some(entity) = runtime_node.entity else {
                continue;
            };

            if let Ok((_, mut content_size, mut computed_layout)) = layout_query.get_mut(entity) {
                content_size.bypass_change_detection().measure = runtime_node.measure;
                computed_layout
                    .bypass_change_detection()
                    .set(runtime_node.unrounded_layout, runtime_node.final_layout);
            }
        }

        Ok(())
    }
}

fn build_runtime_layout_tree<'a>(
    entity: Entity,
    ui_children: &UiChildren,
    node_query: &'a Query<(Entity, &Node, &ComputedUiRenderTargetInfo)>,
    layout_query: &mut Query<(Entity, &mut ContentSize, &mut ComputedLayout)>,
    runtime_nodes: &mut HashMap<NodeId, RuntimeLayoutNode<'a>>,
) -> Result<bool, LayoutError> {
    let mut child_ids = Vec::new();
    for child in ui_children.iter_ui_children(entity) {
        if build_runtime_layout_tree(child, ui_children, node_query, layout_query, runtime_nodes)? {
            child_ids.push(entity_node_id(child));
        }
    }

    let Ok((_, node, computed_target)) = node_query.get(entity) else {
        return Ok(false);
    };

    let Ok((_, mut content_size, _)) = layout_query.get_mut(entity) else {
        return Ok(false);
    };

    let layout_context = LayoutContext::new(
        computed_target.scale_factor(),
        computed_target.physical_size().as_vec2(),
    );

    runtime_nodes.insert(
        entity_node_id(entity),
        RuntimeLayoutNode {
            entity: Some(entity),
            style: CoreNode::from_node(node, layout_context),
            cache: Cache::new(),
            unrounded_layout: Layout::new(),
            final_layout: Layout::new(),
            measure: content_size.bypass_change_detection().measure.take(),
            children: child_ids,
        },
    );

    Ok(true)
}

struct RuntimeLayoutNode<'a> {
    entity: Option<Entity>,
    style: CoreNode<'a>,
    cache: Cache,
    unrounded_layout: Layout,
    final_layout: Layout,
    measure: Option<NodeMeasure>,
    children: Vec<NodeId>,
}

impl<'a> RuntimeLayoutNode<'a> {
    fn viewport(root_node_id: NodeId) -> Self {
        Self {
            entity: None,
            style: CoreNode::viewport(),
            cache: Cache::new(),
            unrounded_layout: Layout::new(),
            final_layout: Layout::new(),
            measure: None,
            children: vec![root_node_id],
        }
    }
}

struct EcsLayoutTree<'a, 'node> {
    nodes: HashMap<NodeId, RuntimeLayoutNode<'node>>,
    measure_function: &'a mut dyn FnMut(
        taffy::Size<Option<f32>>,
        taffy::Size<AvailableSpace>,
        &mut NodeMeasure,
        &Style,
    ) -> taffy::Size<f32>,
}

impl TraversePartialTree for EcsLayoutTree<'_, '_> {
    type ChildIter<'a>
        = core::iter::Copied<core::slice::Iter<'a, NodeId>>
    where
        Self: 'a;

    fn child_ids(&self, parent_node_id: NodeId) -> Self::ChildIter<'_> {
        self.nodes
            .get(&parent_node_id)
            .expect("missing layout node")
            .children
            .iter()
            .copied()
    }

    fn child_count(&self, parent_node_id: NodeId) -> usize {
        self.nodes
            .get(&parent_node_id)
            .expect("missing layout node")
            .children
            .len()
    }

    fn get_child_id(&self, parent_node_id: NodeId, child_index: usize) -> NodeId {
        self.nodes
            .get(&parent_node_id)
            .expect("missing layout node")
            .children[child_index]
    }
}

impl TraverseTree for EcsLayoutTree<'_, '_> {}

impl<'tree, 'node> LayoutPartialTree for EcsLayoutTree<'tree, 'node> {
    type CoreContainerStyle<'a>
        = &'a CoreNode<'node>
    where
        Self: 'a;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        &self.nodes.get(&node_id).expect("missing layout node").style
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.nodes
            .get_mut(&node_id)
            .expect("missing layout node")
            .unrounded_layout = *layout;
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
                .style
                .clone();
            let has_children = tree.child_count(node_id) > 0;

            match (style.display(), has_children) {
                (Display::None, _) => compute_hidden_layout(tree, node_id),
                (Display::Block, true) => compute_block_layout(tree, node_id, inputs, None),
                (Display::Flex, true) => compute_flexbox_layout(tree, node_id, inputs),
                (Display::Grid, true) => compute_grid_layout(tree, node_id, inputs),
                (_, false) => compute_leaf_layout(
                    inputs,
                    &style,
                    |_, _| 0.0,
                    |known_dimensions, available_space| {
                        let taffy_style = style.to_taffy_style();
                        let Some(mut measure) = tree
                            .nodes
                            .get_mut(&node_id)
                            .expect("missing layout node")
                            .measure
                            .take()
                        else {
                            return taffy::Size::ZERO;
                        };
                        let measured = (tree.measure_function)(
                            known_dimensions,
                            available_space,
                            &mut measure,
                            &taffy_style,
                        );
                        tree.nodes
                            .get_mut(&node_id)
                            .expect("missing layout node")
                            .measure = Some(measure);
                        measured
                    },
                ),
            }
        })
    }
}

impl CacheTree for EcsLayoutTree<'_, '_> {
    fn cache_get(&self, node_id: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.nodes
            .get(&node_id)
            .expect("missing layout node")
            .cache
            .get(input)
    }

    fn cache_store(&mut self, node_id: NodeId, input: &LayoutInput, layout_output: LayoutOutput) {
        self.nodes
            .get_mut(&node_id)
            .expect("missing layout node")
            .cache
            .store(input, layout_output);
    }

    fn cache_clear(&mut self, node_id: NodeId) {
        self.nodes
            .get_mut(&node_id)
            .expect("missing layout node")
            .cache
            .clear();
    }
}

impl<'tree, 'node> LayoutBlockContainer for EcsLayoutTree<'tree, 'node> {
    type BlockContainerStyle<'a>
        = &'a CoreNode<'node>
    where
        Self: 'a;

    type BlockItemStyle<'a>
        = &'a CoreNode<'node>
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl<'tree, 'node> LayoutFlexboxContainer for EcsLayoutTree<'tree, 'node> {
    type FlexboxContainerStyle<'a>
        = &'a CoreNode<'node>
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = &'a CoreNode<'node>
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl<'tree, 'node> LayoutGridContainer for EcsLayoutTree<'tree, 'node> {
    type GridContainerStyle<'a>
        = &'a CoreNode<'node>
    where
        Self: 'a;

    type GridItemStyle<'a>
        = &'a CoreNode<'node>
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: NodeId) -> Self::GridContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: NodeId) -> Self::GridItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl RoundTree for EcsLayoutTree<'_, '_> {
    fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
        self.nodes
            .get(&node_id)
            .expect("missing layout node")
            .unrounded_layout
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.nodes
            .get_mut(&node_id)
            .expect("missing layout node")
            .final_layout = *layout;
    }
}

pub fn get_text_buffer<'a>(
    needs_buffer: bool,
    ctx: &mut NodeMeasure,
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
    fn test_initialization() {
        let _ui_surface = UiSurface;
    }

    #[test]
    fn missing_layout_returns_none() {
        let computed_layout = ComputedLayout::default();
        assert!(computed_layout.get(true).is_none());
    }
}
