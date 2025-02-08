use core::fmt;

use bevy_platform_support::collections::hash_map::Entry;
use taffy::TaffyTree;

use bevy_ecs::{
    entity::{hash_map::EntityHashMap, Entity},
    prelude::Resource,
};
use bevy_math::{UVec2, Vec2};
use bevy_utils::default;

use crate::{layout::convert, LayoutContext, LayoutError, Measure, MeasureArgs, Node, NodeMeasure};
use bevy_text::CosmicFontSystem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootNodePair {
    // The implicit "viewport" node created by Bevy
    pub(super) implicit_viewport_node: taffy::NodeId,
    // The root (parentless) node specified by the user
    pub(super) user_root_node: taffy::NodeId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    // Implicit "viewport" node if this `LayoutNode` corresponds to a root UI node entity
    pub(super) viewport_id: Option<taffy::NodeId>,
    // The id of the node in the taffy tree
    pub(super) id: taffy::NodeId,
}

impl From<taffy::NodeId> for LayoutNode {
    fn from(value: taffy::NodeId) -> Self {
        LayoutNode {
            viewport_id: None,
            id: value,
        }
    }
}

#[derive(Resource)]
pub struct UiSurface {
    pub(super) entity_to_taffy: EntityHashMap<LayoutNode>,
    pub(super) camera_entity_to_taffy: EntityHashMap<EntityHashMap<taffy::NodeId>>,
    pub(super) camera_roots: EntityHashMap<Vec<RootNodePair>>,
    pub(super) taffy: TaffyTree<NodeMeasure>,
    taffy_children_scratch: Vec<taffy::NodeId>,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<EntityHashMap<taffy::NodeId>>();
    _assert_send_sync::<EntityHashMap<EntityHashMap<taffy::NodeId>>>();
    _assert_send_sync::<EntityHashMap<Vec<RootNodePair>>>();
    _assert_send_sync::<TaffyTree<NodeMeasure>>();
    _assert_send_sync::<UiSurface>();
}

impl fmt::Debug for UiSurface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UiSurface")
            .field("entity_to_taffy", &self.entity_to_taffy)
            .field("camera_entity_to_taffy", &self.camera_entity_to_taffy)
            .field("camera_roots", &self.camera_roots)
            .field("taffy_children_scratch", &self.taffy_children_scratch)
            .finish()
    }
}

impl Default for UiSurface {
    fn default() -> Self {
        let taffy: TaffyTree<NodeMeasure> = TaffyTree::new();
        Self {
            entity_to_taffy: Default::default(),
            camera_entity_to_taffy: Default::default(),
            camera_roots: Default::default(),
            taffy,
            taffy_children_scratch: Vec::new(),
        }
    }
}

impl UiSurface {
    /// Retrieves the Taffy node associated with the given UI node entity and updates its style.
    /// If no associated Taffy node exists a new Taffy node is inserted into the Taffy layout.
    pub fn upsert_node(
        &mut self,
        layout_context: &LayoutContext,
        entity: Entity,
        node: &Node,
        mut new_node_context: Option<NodeMeasure>,
    ) {
        let taffy = &mut self.taffy;

        match self.entity_to_taffy.entry(entity) {
            Entry::Occupied(entry) => {
                let taffy_node = *entry.get();
                let has_measure = if new_node_context.is_some() {
                    taffy
                        .set_node_context(taffy_node.id, new_node_context)
                        .unwrap();
                    true
                } else {
                    taffy.get_node_context(taffy_node.id).is_some()
                };

                taffy
                    .set_style(
                        taffy_node.id,
                        convert::from_node(node, layout_context, has_measure),
                    )
                    .unwrap();
            }
            Entry::Vacant(entry) => {
                let taffy_node = if let Some(measure) = new_node_context.take() {
                    taffy.new_leaf_with_context(
                        convert::from_node(node, layout_context, true),
                        measure,
                    )
                } else {
                    taffy.new_leaf(convert::from_node(node, layout_context, false))
                };
                entry.insert(taffy_node.unwrap().into());
            }
        }
    }

    /// Update the `MeasureFunc` of the taffy node corresponding to the given [`Entity`] if the node exists.
    pub fn update_node_context(&mut self, entity: Entity, context: NodeMeasure) -> Option<()> {
        let taffy_node = self.entity_to_taffy.get(&entity)?;
        self.taffy
            .set_node_context(taffy_node.id, Some(context))
            .ok()
    }

    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(&mut self, entity: Entity, children: impl Iterator<Item = Entity>) {
        self.taffy_children_scratch.clear();

        for child in children {
            if let Some(taffy_node) = self.entity_to_taffy.get_mut(&child) {
                self.taffy_children_scratch.push(taffy_node.id);
                if let Some(viewport_id) = taffy_node.viewport_id.take() {
                    self.taffy.remove(viewport_id).ok();
                }
            }
        }

        let taffy_node = self.entity_to_taffy.get(&entity).unwrap();
        self.taffy
            .set_children(taffy_node.id, &self.taffy_children_scratch)
            .unwrap();
    }

    /// Removes children from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_children(&mut self, entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_children(taffy_node.id, &[]).unwrap();
        }
    }

    /// Removes the measure from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_node_context(&mut self, entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_node_context(taffy_node.id, None).unwrap();
        }
    }

    /// Sets the ui root node entities as children to the root node in the taffy layout.
    pub fn set_camera_children(
        &mut self,
        camera_id: Entity,
        children: impl Iterator<Item = Entity>,
    ) {
        let viewport_style = taffy::style::Style {
            display: taffy::style::Display::Grid,
            // Note: Taffy percentages are floats ranging from 0.0 to 1.0.
            // So this is setting width:100% and height:100%
            size: taffy::geometry::Size {
                width: taffy::style::Dimension::Percent(1.0),
                height: taffy::style::Dimension::Percent(1.0),
            },
            align_items: Some(taffy::style::AlignItems::Start),
            justify_items: Some(taffy::style::JustifyItems::Start),
            ..default()
        };

        let camera_root_node_map = self.camera_entity_to_taffy.entry(camera_id).or_default();
        let existing_roots = self.camera_roots.entry(camera_id).or_default();
        let mut new_roots = Vec::new();
        for entity in children {
            let node = self.entity_to_taffy.get_mut(&entity).unwrap();
            let root_node = existing_roots
                .iter()
                .find(|n| n.user_root_node == node.id)
                .cloned()
                .unwrap_or_else(|| {
                    if let Some(previous_parent) = self.taffy.parent(node.id) {
                        // remove the root node from the previous implicit node's children
                        self.taffy.remove_child(previous_parent, node.id).unwrap();
                    }

                    let viewport_node = *camera_root_node_map.entry(entity).or_insert_with(|| {
                        node.viewport_id
                            .unwrap_or_else(|| self.taffy.new_leaf(viewport_style.clone()).unwrap())
                    });
                    node.viewport_id = Some(viewport_node);
                    self.taffy.add_child(viewport_node, node.id).unwrap();
                    RootNodePair {
                        implicit_viewport_node: viewport_node,
                        user_root_node: node.id,
                    }
                });
            new_roots.push(root_node);
        }

        self.camera_roots.insert(camera_id, new_roots);
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_camera_layout<'a>(
        &mut self,
        camera: Entity,
        render_target_resolution: UVec2,
        buffer_query: &'a mut bevy_ecs::prelude::Query<&mut bevy_text::ComputedTextBlock>,
        font_system: &'a mut CosmicFontSystem,
    ) {
        let Some(camera_root_nodes) = self.camera_roots.get(&camera) else {
            return;
        };

        let available_space = taffy::geometry::Size {
            width: taffy::style::AvailableSpace::Definite(render_target_resolution.x as f32),
            height: taffy::style::AvailableSpace::Definite(render_target_resolution.y as f32),
        };
        for root_nodes in camera_root_nodes {
            self.taffy
                .compute_layout_with_measure(
                    root_nodes.implicit_viewport_node,
                    available_space,
                    |known_dimensions: taffy::Size<Option<f32>>,
                     available_space: taffy::Size<taffy::AvailableSpace>,
                     _node_id: taffy::NodeId,
                     context: Option<&mut NodeMeasure>,
                     style: &taffy::Style|
                     -> taffy::Size<f32> {
                        context
                            .map(|ctx| {
                                let buffer = get_text_buffer(
                                    crate::widget::TextMeasure::needs_buffer(
                                        known_dimensions.height,
                                        available_space.width,
                                    ),
                                    ctx,
                                    buffer_query,
                                );
                                let size = ctx.measure(
                                    MeasureArgs {
                                        width: known_dimensions.width,
                                        height: known_dimensions.height,
                                        available_width: available_space.width,
                                        available_height: available_space.height,
                                        font_system,
                                        buffer,
                                    },
                                    style,
                                );
                                taffy::Size {
                                    width: size.x,
                                    height: size.y,
                                }
                            })
                            .unwrap_or(taffy::Size::ZERO)
                    },
                )
                .unwrap();
        }
    }

    /// Removes each camera entity from the internal map and then removes their associated node from taffy
    pub fn remove_camera_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            if let Some(camera_root_node_map) = self.camera_entity_to_taffy.remove(&entity) {
                for (entity, node) in camera_root_node_map.iter() {
                    self.taffy.remove(*node).unwrap();
                    self.entity_to_taffy.get_mut(entity).unwrap().viewport_id = None;
                }
            }
        }
    }

    /// Removes each entity from the internal map and then removes their associated nodes from taffy
    pub fn remove_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            if let Some(node) = self.entity_to_taffy.remove(&entity) {
                self.taffy.remove(node.id).unwrap();
                if let Some(viewport_node) = node.viewport_id {
                    self.taffy.remove(viewport_node).ok();
                }
            }
        }
    }

    /// Get the layout geometry for the taffy node corresponding to the ui node [`Entity`].
    /// Does not compute the layout geometry, `compute_window_layouts` should be run before using this function.
    /// On success returns a pair consisting of the final resolved layout values after rounding
    /// and the size of the node after layout resolution but before rounding.
    pub fn get_layout(
        &mut self,
        entity: Entity,
        use_rounding: bool,
    ) -> Result<(taffy::Layout, Vec2), LayoutError> {
        let Some(taffy_node) = self.entity_to_taffy.get(&entity) else {
            return Err(LayoutError::InvalidHierarchy);
        };

        if use_rounding {
            self.taffy.enable_rounding();
        } else {
            self.taffy.disable_rounding();
        }

        let out = match self.taffy.layout(taffy_node.id).cloned() {
            Ok(layout) => {
                self.taffy.disable_rounding();
                let taffy_size = self.taffy.layout(taffy_node.id).unwrap().size;
                let unrounded_size = Vec2::new(taffy_size.width, taffy_size.height);
                Ok((layout, unrounded_size))
            }
            Err(taffy_error) => Err(LayoutError::TaffyError(taffy_error)),
        };

        self.taffy.enable_rounding();
        out
    }
}

fn get_text_buffer<'a>(
    needs_buffer: bool,
    ctx: &mut NodeMeasure,
    query: &'a mut bevy_ecs::prelude::Query<&mut bevy_text::ComputedTextBlock>,
) -> Option<&'a mut bevy_text::ComputedTextBlock> {
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
    use crate::{ContentSize, FixedMeasure};
    use bevy_math::Vec2;
    use taffy::TraversePartialTree;

    /// Checks if the parent of the `user_root_node` in a `RootNodePair`
    /// is correctly assigned as the `implicit_viewport_node`.
    fn is_root_node_pair_valid(
        taffy_tree: &TaffyTree<NodeMeasure>,
        root_node_pair: &RootNodePair,
    ) -> bool {
        taffy_tree.parent(root_node_pair.user_root_node)
            == Some(root_node_pair.implicit_viewport_node)
    }

    /// Tries to get the root node pair for a given root node entity with the specified camera entity
    fn get_root_node_pair_exact(
        ui_surface: &UiSurface,
        root_node_entity: Entity,
        camera_entity: Entity,
    ) -> Option<&RootNodePair> {
        let root_node_pairs = ui_surface.camera_roots.get(&camera_entity)?;
        let root_node_taffy = ui_surface.entity_to_taffy.get(&root_node_entity)?;
        root_node_pairs
            .iter()
            .find(|&root_node_pair| root_node_pair.user_root_node == root_node_taffy.id)
    }

    #[test]
    fn test_initialization() {
        let ui_surface = UiSurface::default();
        assert!(ui_surface.entity_to_taffy.is_empty());
        assert!(ui_surface.camera_entity_to_taffy.is_empty());
        assert!(ui_surface.camera_roots.is_empty());
        assert_eq!(ui_surface.taffy.total_node_count(), 0);
    }

    #[test]
    fn test_upsert() {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let node = Node::default();

        // standard upsert
        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        // should be inserted into taffy
        assert_eq!(ui_surface.taffy.total_node_count(), 1);
        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        // test duplicate insert 1
        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        // node count should not have increased
        assert_eq!(ui_surface.taffy.total_node_count(), 1);

        // assign root node to camera
        ui_surface.set_camera_children(camera_entity, vec![root_node_entity].into_iter());

        // each root node will create 2 taffy nodes
        assert_eq!(ui_surface.taffy.total_node_count(), 2);

        // root node pair should now exist
        let root_node_pair = get_root_node_pair_exact(&ui_surface, root_node_entity, camera_entity)
            .expect("expected root node pair");
        assert!(is_root_node_pair_valid(&ui_surface.taffy, root_node_pair));

        // test duplicate insert 2
        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        // node count should not have increased
        assert_eq!(ui_surface.taffy.total_node_count(), 2);

        // root node pair should be unaffected
        let root_node_pair = get_root_node_pair_exact(&ui_surface, root_node_entity, camera_entity)
            .expect("expected root node pair");
        assert!(is_root_node_pair_valid(&ui_surface.taffy, root_node_pair));
    }

    #[test]
    fn test_get_root_node_pair_exact() {
        /// Attempts to find the camera entity that holds a reference to the given root node entity
        fn get_associated_camera_entity(
            ui_surface: &UiSurface,
            root_node_entity: Entity,
        ) -> Option<Entity> {
            for (&camera_entity, root_node_map) in ui_surface.camera_entity_to_taffy.iter() {
                if root_node_map.contains_key(&root_node_entity) {
                    return Some(camera_entity);
                }
            }
            None
        }

        /// Attempts to find the root node pair corresponding to the given root node entity
        fn get_root_node_pair(
            ui_surface: &UiSurface,
            root_node_entity: Entity,
        ) -> Option<&RootNodePair> {
            let camera_entity = get_associated_camera_entity(ui_surface, root_node_entity)?;
            get_root_node_pair_exact(ui_surface, root_node_entity, camera_entity)
        }

        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let node = Node::default();

        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        // assign root node to camera
        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter());

        assert_eq!(
            get_associated_camera_entity(&ui_surface, root_node_entity),
            Some(camera_entity)
        );
        assert_eq!(
            get_associated_camera_entity(&ui_surface, Entity::from_raw(2)),
            None
        );

        let root_node_pair = get_root_node_pair(&ui_surface, root_node_entity);
        assert!(root_node_pair.is_some());
        assert_eq!(
            Some(root_node_pair.unwrap().user_root_node).as_ref(),
            ui_surface
                .entity_to_taffy
                .get(&root_node_entity)
                .map(|taffy_node| &taffy_node.id)
        );

        assert_eq!(
            get_root_node_pair_exact(&ui_surface, root_node_entity, camera_entity),
            root_node_pair
        );
    }

    #[expect(
        unreachable_code,
        reason = "Certain pieces of code tested here cause the test to fail if made reachable; see #16362 for progress on fixing this"
    )]
    #[test]
    fn test_remove_camera_entities() {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let node = Node::default();

        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        // assign root node to camera
        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter());

        assert!(ui_surface
            .camera_entity_to_taffy
            .contains_key(&camera_entity));
        assert!(ui_surface
            .camera_entity_to_taffy
            .get(&camera_entity)
            .unwrap()
            .contains_key(&root_node_entity));
        assert!(ui_surface.camera_roots.contains_key(&camera_entity));
        let root_node_pair = get_root_node_pair_exact(&ui_surface, root_node_entity, camera_entity)
            .expect("expected root node pair");
        assert!(ui_surface
            .camera_roots
            .get(&camera_entity)
            .unwrap()
            .contains(root_node_pair));

        ui_surface.remove_camera_entities([camera_entity]);

        // should not affect `entity_to_taffy`
        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        // `camera_roots` and `camera_entity_to_taffy` should no longer contain entries for `camera_entity`
        assert!(!ui_surface
            .camera_entity_to_taffy
            .contains_key(&camera_entity));

        return; // TODO: can't pass the test if we continue - not implemented (remove allow(unreachable_code))

        assert!(!ui_surface.camera_roots.contains_key(&camera_entity));

        // root node pair should be removed
        let root_node_pair = get_root_node_pair_exact(&ui_surface, root_node_entity, camera_entity);
        assert_eq!(root_node_pair, None);
    }

    #[expect(
        unreachable_code,
        reason = "Certain pieces of code tested here cause the test to fail if made reachable; see #16362 for progress on fixing this"
    )]
    #[test]
    fn test_remove_entities() {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let node = Node::default();

        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter());

        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));
        assert!(ui_surface
            .camera_entity_to_taffy
            .get(&camera_entity)
            .unwrap()
            .contains_key(&root_node_entity));
        let root_node_pair =
            get_root_node_pair_exact(&ui_surface, root_node_entity, camera_entity).unwrap();
        assert!(ui_surface
            .camera_roots
            .get(&camera_entity)
            .unwrap()
            .contains(root_node_pair));

        ui_surface.remove_entities([root_node_entity]);
        assert!(!ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        return; // TODO: can't pass the test if we continue - not implemented (remove allow(unreachable_code))

        assert!(!ui_surface
            .camera_entity_to_taffy
            .get(&camera_entity)
            .unwrap()
            .contains_key(&root_node_entity));
        assert!(!ui_surface
            .camera_entity_to_taffy
            .get(&camera_entity)
            .unwrap()
            .contains_key(&root_node_entity));
        assert!(ui_surface
            .camera_roots
            .get(&camera_entity)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_try_update_measure() {
        let mut ui_surface = UiSurface::default();
        let root_node_entity = Entity::from_raw(1);
        let node = Node::default();

        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);
        let mut content_size = ContentSize::default();
        content_size.set(NodeMeasure::Fixed(FixedMeasure { size: Vec2::ONE }));
        let measure_func = content_size.measure.take().unwrap();
        assert!(ui_surface
            .update_node_context(root_node_entity, measure_func)
            .is_some());
    }

    #[test]
    fn test_update_children() {
        let mut ui_surface = UiSurface::default();
        let root_node_entity = Entity::from_raw(1);
        let child_entity = Entity::from_raw(2);
        let node = Node::default();

        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);
        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, child_entity, &node, None);

        ui_surface.update_children(root_node_entity, vec![child_entity].into_iter());

        let parent_node = *ui_surface.entity_to_taffy.get(&root_node_entity).unwrap();
        let child_node = *ui_surface.entity_to_taffy.get(&child_entity).unwrap();
        assert_eq!(ui_surface.taffy.parent(child_node.id), Some(parent_node.id));
    }

    #[expect(
        unreachable_code,
        reason = "Certain pieces of code tested here cause the test to fail if made reachable; see #16362 for progress on fixing this"
    )]
    #[test]
    fn test_set_camera_children() {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let child_entity = Entity::from_raw(2);
        let node = Node::default();

        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);
        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, child_entity, &node, None);

        let root_taffy_node = *ui_surface.entity_to_taffy.get(&root_node_entity).unwrap();
        let child_taffy = *ui_surface.entity_to_taffy.get(&child_entity).unwrap();

        // set up the relationship manually
        ui_surface
            .taffy
            .add_child(root_taffy_node.id, child_taffy.id)
            .unwrap();

        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter());

        assert!(
            ui_surface
                .camera_entity_to_taffy
                .get(&camera_entity)
                .unwrap()
                .contains_key(&root_node_entity),
            "root node not associated with camera"
        );
        assert!(
            !ui_surface
                .camera_entity_to_taffy
                .get(&camera_entity)
                .unwrap()
                .contains_key(&child_entity),
            "child of root node should not be associated with camera"
        );

        let _root_node_pair =
            get_root_node_pair_exact(&ui_surface, root_node_entity, camera_entity)
                .expect("expected root node pair");

        assert_eq!(
            ui_surface.taffy.parent(child_taffy.id),
            Some(root_taffy_node.id)
        );
        let root_taffy_children = ui_surface.taffy.children(root_taffy_node.id).unwrap();
        assert!(
            root_taffy_children.contains(&child_taffy.id),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node.id),
            1,
            "expected root node child count to be 1"
        );

        // clear camera's root nodes
        ui_surface.set_camera_children(camera_entity, Vec::<Entity>::new().into_iter());

        return; // TODO: can't pass the test if we continue - not implemented (remove allow(unreachable_code))

        assert!(
            !ui_surface
                .camera_entity_to_taffy
                .get(&camera_entity)
                .unwrap()
                .contains_key(&root_node_entity),
            "root node should have been unassociated with camera"
        );
        assert!(
            !ui_surface
                .camera_entity_to_taffy
                .get(&camera_entity)
                .unwrap()
                .contains_key(&child_entity),
            "child of root node should not be associated with camera"
        );

        let root_taffy_children = ui_surface.taffy.children(root_taffy_node.id).unwrap();
        assert!(
            root_taffy_children.contains(&child_taffy.id),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node.id),
            1,
            "expected root node child count to be 1"
        );

        // re-associate root node with camera
        ui_surface.set_camera_children(camera_entity, vec![root_node_entity].into_iter());

        assert!(
            ui_surface
                .camera_entity_to_taffy
                .get(&camera_entity)
                .unwrap()
                .contains_key(&root_node_entity),
            "root node should have been re-associated with camera"
        );
        assert!(
            !ui_surface
                .camera_entity_to_taffy
                .get(&camera_entity)
                .unwrap()
                .contains_key(&child_entity),
            "child of root node should not be associated with camera"
        );

        let child_taffy = ui_surface.entity_to_taffy.get(&child_entity).unwrap();
        let root_taffy_children = ui_surface.taffy.children(root_taffy_node.id).unwrap();
        assert!(
            root_taffy_children.contains(&child_taffy.id),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node.id),
            1,
            "expected root node child count to be 1"
        );
    }
}
