use core::fmt;

use taffy::TaffyTree;

use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    prelude::Resource,
};
use bevy_math::{UVec2, Vec2};
use bevy_utils::default;

use crate::{layout::convert, LayoutContext, LayoutError, Measure, MeasureArgs, Node, NodeMeasure};
use bevy_text::CosmicFontSystem;

#[inline(always)]
/// Style used for `implicit_viewport_node`
fn default_viewport_style() -> taffy::style::Style {
    taffy::style::Style {
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
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stores reference data to quickly identify:
/// - Its associated camera
/// - Its parent `implicit_viewport_node` taffy node
///
/// see: [`super::UiRootNodes`] for explanation on what a "root node" is
pub struct RootNodeData {
    /// Associated camera `Entity`
    ///
    /// inferred by components: `TargetCamera`, `IsDefaultUiCamera`
    ///
    /// "Orphans" are root nodes not assigned to a camera.
    /// Root nodes might temporarily enter an orphan state as they transition between cameras
    /// The reason for this is to prevent us from prematurely recreating taffy nodes
    /// and allowing for the entities to be cleaned up when they are requested to be removed by the ECS
    pub(super) camera_entity: Option<Entity>,
    /// The implicit "viewport" node created by Bevy
    ///
    /// This forces the root nodes to behave independently to other root nodes.
    /// Just as if they were set to `PositionType::Absolute`
    ///
    /// This must be manually removed on `Entity` despawn
    /// or else it will survive in the taffy tree with no references
    pub(super) implicit_viewport_node: taffy::NodeId,
}

#[derive(Resource)]
/// Manages state and hierarchy for ui entities
pub struct UiSurface {
    pub(super) entity_to_taffy: EntityHashMap<taffy::NodeId>,
    /// Maps root ui node `Entity` to its corresponding `RootNodeData`
    pub(super) root_node_data: EntityHashMap<RootNodeData>,
    /// Maps camera `Entity` to an associated `EntityHashSet` of root ui nodes
    pub(super) camera_root_nodes: EntityHashMap<EntityHashSet>,
    /// Manages the UI Node Tree
    pub(super) taffy: TaffyTree<NodeMeasure>,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<EntityHashMap<taffy::NodeId>>();
    _assert_send_sync::<EntityHashMap<RootNodeData>>();
    _assert_send_sync::<EntityHashMap<EntityHashSet>>();
    _assert_send_sync::<TaffyTree<NodeMeasure>>();
    _assert_send_sync::<UiSurface>();
}

impl fmt::Debug for UiSurface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UiSurface")
            .field("entity_to_taffy", &self.entity_to_taffy)
            .field("root_node_data", &self.root_node_data)
            .field("camera_root_nodes", &self.camera_root_nodes)
            .finish()
    }
}

impl Default for UiSurface {
    fn default() -> Self {
        let taffy: TaffyTree<NodeMeasure> = TaffyTree::new();
        Self {
            entity_to_taffy: Default::default(),
            root_node_data: Default::default(),
            camera_root_nodes: Default::default(),
            taffy,
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

        let mut added = false;
        let taffy_node_id = *self.entity_to_taffy.entry(entity).or_insert_with(|| {
            added = true;
            if let Some(measure) = new_node_context.take() {
                taffy
                    .new_leaf_with_context(convert::from_node(node, layout_context, true), measure)
                    .unwrap()
            } else {
                taffy
                    .new_leaf(convert::from_node(node, layout_context, false))
                    .unwrap()
            }
        });

        if !added {
            let has_measure = if new_node_context.is_some() {
                taffy
                    .set_node_context(taffy_node_id, new_node_context)
                    .unwrap();
                true
            } else {
                taffy.get_node_context(taffy_node_id).is_some()
            };

            taffy
                .set_style(
                    taffy_node_id,
                    convert::from_node(node, layout_context, has_measure),
                )
                .unwrap();
        }
    }

    /// Update the `MeasureFunc` of the taffy node corresponding to the given [`Entity`] if the node exists.
    pub fn update_node_context(&mut self, entity: Entity, context: NodeMeasure) -> Option<()> {
        let taffy_node = self.entity_to_taffy.get(&entity)?;
        self.taffy.set_node_context(*taffy_node, Some(context)).ok()
    }

    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(&mut self, entity: Entity, children: impl Iterator<Item = Entity>) {
        let children = children
            .map(|child| {
                self.entity_to_taffy
                    .get(&child)
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!("failed to resolve taffy id for child entity {child} in {entity}")
                    })
            })
            .collect::<Vec<_>>();

        let taffy_node = self.entity_to_taffy.get(&entity).unwrap();
        self.taffy.set_children(*taffy_node, &children).unwrap();
    }

    /// Removes children from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_children(&mut self, entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_children(*taffy_node, &[]).unwrap();
        }
    }

    /// Removes the measure from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_node_context(&mut self, entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_node_context(*taffy_node, None).unwrap();
        }
    }

    /// Removes camera association to root node
    /// Shorthand for calling `replace_camera_association(root_node_entity, None)`
    fn mark_root_node_as_orphaned(&mut self, root_node_entity: Entity) {
        self.replace_camera_association(root_node_entity, None);
    }

    /// Reassigns or removes a root node's associated camera entity
    /// `Some(camera_entity)` - Updates camera association to root node
    /// `None` - Removes camera association to root node
    /// Does not check to see if they are the same before performing operations
    fn replace_camera_association(
        &mut self,
        root_node_entity: Entity,
        new_camera_entity_option: Option<Entity>,
    ) {
        if let Some(root_node_data) = self.root_node_data.get_mut(&root_node_entity) {
            // Clear existing camera association, if any
            if let Some(old_camera_entity) = root_node_data.camera_entity.take() {
                let prev_camera_root_nodes = self.camera_root_nodes.get_mut(&old_camera_entity);
                if let Some(prev_camera_root_nodes) = prev_camera_root_nodes {
                    prev_camera_root_nodes.remove(&root_node_entity);
                }
            }

            // Establish new camera association, if provided
            if let Some(camera_entity) = new_camera_entity_option {
                root_node_data.camera_entity.replace(camera_entity);
                self.camera_root_nodes
                    .entry(camera_entity)
                    .or_default()
                    .insert(root_node_entity);
            }
        }
    }

    /// Creates or updates a root node
    fn create_or_update_root_node_data(
        &mut self,
        root_node_entity: Entity,
        camera_entity: Entity,
    ) -> &mut RootNodeData {
        let user_root_node = *self.entity_to_taffy.get(&root_node_entity).expect("create_or_update_root_node_data called before root_node_entity was added to taffy tree or was previously removed");

        let mut added = false;

        // creates mutable borrow on self that lives as long as the result
        let _ = self
            .root_node_data
            .entry(root_node_entity)
            .or_insert_with(|| {
                added = true;

                self.camera_root_nodes
                    .entry(camera_entity)
                    .or_default()
                    .insert(root_node_entity);

                let implicit_viewport_node = self.taffy.new_leaf(default_viewport_style()).unwrap();

                self.taffy
                    .add_child(implicit_viewport_node, user_root_node)
                    .unwrap();

                RootNodeData {
                    camera_entity: Some(camera_entity),
                    implicit_viewport_node,
                }
            });

        if !added {
            self.replace_camera_association(root_node_entity, Some(camera_entity));
        }

        self.root_node_data
            .get_mut(&root_node_entity)
            .unwrap_or_else(|| unreachable!())
    }

    /// Sets the ui root node entities as children to the root node in the taffy layout.
    pub fn set_camera_children(
        &mut self,
        camera_entity: Entity,
        children: impl Iterator<Item = Entity>,
    ) {
        let removed_children = self.camera_root_nodes.entry(camera_entity).or_default();
        let mut removed_children = removed_children.clone();

        for ui_entity in children {
            // creates mutable borrow on self that lives as long as the result
            let _ = self.create_or_update_root_node_data(ui_entity, camera_entity);

            // drop the mutable borrow on self by re-fetching
            let root_node_data = self
                .root_node_data
                .get(&ui_entity)
                .unwrap_or_else(|| unreachable!());

            // fix taffy relationships
            {
                let taffy_node = *self.entity_to_taffy.get(&ui_entity).unwrap();
                if let Some(parent) = self.taffy.parent(taffy_node) {
                    self.taffy.remove_child(parent, taffy_node).unwrap();
                }
                self.taffy
                    .add_child(root_node_data.implicit_viewport_node, taffy_node)
                    .unwrap();
            }
            removed_children.remove(&ui_entity);
        }

        for &orphan in removed_children.iter() {
            self.remove_root_node_viewport(orphan);
        }
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_camera_layout<'a>(
        &mut self,
        camera_entity: Entity,
        render_target_resolution: UVec2,
        buffer_query: &'a mut bevy_ecs::prelude::Query<&mut bevy_text::ComputedTextBlock>,
        font_system: &'a mut CosmicFontSystem,
    ) {
        let Some(camera_root_nodes) = self.camera_root_nodes.get(&camera_entity) else {
            return;
        };

        let available_space = taffy::geometry::Size {
            width: taffy::style::AvailableSpace::Definite(render_target_resolution.x as f32),
            height: taffy::style::AvailableSpace::Definite(render_target_resolution.y as f32),
        };

        for root_node_entity in camera_root_nodes {
            let root_node_data = self
                .root_node_data
                .get(root_node_entity)
                .expect("root_node_data missing");

            if root_node_data.camera_entity.is_none() {
                panic!("internal map out of sync");
            }
            self.taffy
                .compute_layout_with_measure(
                    root_node_data.implicit_viewport_node,
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

    /// Disassociates the camera from all of its assigned root nodes and removes their viewport nodes
    /// Removes entry in `camera_root_nodes`
    pub(super) fn remove_camera(&mut self, camera_entity: Entity) {
        if let Some(root_node_entities) = self.camera_root_nodes.remove(&camera_entity) {
            for root_node_entity in root_node_entities {
                self.remove_root_node_viewport(root_node_entity);
            }
        };
    }

    /// Disassociates the root node from the assigned camera (if any) and removes the viewport node from taffy
    /// Removes entry in `root_node_data`
    fn remove_root_node_viewport(&mut self, root_node_entity: Entity) {
        self.mark_root_node_as_orphaned(root_node_entity);
        if let Some(removed) = self.root_node_data.remove(&root_node_entity) {
            self.taffy.remove(removed.implicit_viewport_node).unwrap();
        }
    }

    /// Removes the ui node from the taffy tree, and if it's a root node it also calls `remove_root_node_viewport`
    pub(super) fn remove_ui_node(&mut self, ui_node_entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.remove(&ui_node_entity) {
            self.taffy.remove(taffy_node).unwrap();
        }
        // remove root node entry if this is a root node
        if self.root_node_data.contains_key(&ui_node_entity) {
            self.remove_root_node_viewport(ui_node_entity);
        }
    }

    /// Removes specified camera entities by disassociating them from their associated `implicit_viewport_node`
    /// in the internal map, and subsequently removes the `implicit_viewport_node`
    /// from the `taffy` layout engine for each.
    pub fn remove_camera_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            self.remove_camera(entity);
        }
    }

    /// Removes the specified entities from the internal map while
    /// removing their `implicit_viewport_node` from taffy,
    /// and then subsequently removes their entry from `entity_to_taffy` and associated node from taffy
    pub fn remove_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            self.remove_ui_node(entity);
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

        let out = match self.taffy.layout(*taffy_node).cloned() {
            Ok(layout) => {
                self.taffy.disable_rounding();
                let taffy_size = self.taffy.layout(*taffy_node).unwrap().size;
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

    /// Checks if the parent of the `user_root_node` in a `RootNodeData`
    /// is correctly assigned as the `implicit_viewport_node`.
    fn has_valid_root_node_data(ui_surface: &UiSurface, root_node_entity: &Entity) -> bool {
        let Some(&root_node_taffy_node_id) = ui_surface.entity_to_taffy.get(root_node_entity)
        else {
            return false;
        };
        let Some(root_node_data) = ui_surface.root_node_data.get(root_node_entity) else {
            return false;
        };
        ui_surface.taffy.parent(root_node_taffy_node_id)
            == Some(root_node_data.implicit_viewport_node)
    }

    /// Tries to get the root node data for a given root node entity
    /// and asserts it matches the provided camera entity
    fn get_root_node_data_exact(
        ui_surface: &UiSurface,
        root_node_entity: Entity,
        camera_entity: Entity,
    ) -> Option<&RootNodeData> {
        let root_node_data = ui_surface.root_node_data.get(&root_node_entity)?;
        assert_eq!(root_node_data.camera_entity, Some(camera_entity));
        Some(root_node_data)
    }

    #[test]
    fn test_initialization() {
        let ui_surface = UiSurface::default();
        assert!(ui_surface.entity_to_taffy.is_empty());
        assert!(ui_surface.root_node_data.is_empty());
        assert!(ui_surface.camera_root_nodes.is_empty());
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

        // root node data should now exist
        let _root_node_data =
            get_root_node_data_exact(&ui_surface, root_node_entity, camera_entity)
                .expect("expected root node data");
        assert!(has_valid_root_node_data(&ui_surface, &root_node_entity));

        // test duplicate insert 2
        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        // node count should not have increased
        assert_eq!(ui_surface.taffy.total_node_count(), 2);

        // root node data should be unaffected
        let _root_node_data =
            get_root_node_data_exact(&ui_surface, root_node_entity, camera_entity)
                .expect("expected root node data");
        assert!(has_valid_root_node_data(&ui_surface, &root_node_entity));
    }

    #[test]
    fn test_get_root_node_pair_exact() {
        /// Attempts to find the root node data corresponding to the given root node entity
        fn get_root_node_data(
            ui_surface: &UiSurface,
            root_node_entity: Entity,
        ) -> Option<&RootNodeData> {
            ui_surface.root_node_data.get(&root_node_entity)
        }

        /// Attempts to find the camera entity that holds a reference to the given root node entity
        fn get_associated_camera_entity(
            ui_surface: &UiSurface,
            root_node_entity: Entity,
        ) -> Option<Entity> {
            get_root_node_data(ui_surface, root_node_entity)?.camera_entity
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

        let root_node_data =
            get_root_node_data(&ui_surface, root_node_entity).expect("expected root node data");
        assert_eq!(
            Some(root_node_data),
            ui_surface.root_node_data.get(&root_node_entity),
        );

        assert_eq!(
            get_root_node_data_exact(&ui_surface, root_node_entity, camera_entity),
            Some(root_node_data),
        );
    }

    #[test]
    fn test_remove_camera_entities() {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let node = Node::default();

        ui_surface.upsert_node(&LayoutContext::TEST_CONTEXT, root_node_entity, &node, None);

        // assign root node to camera
        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter());

        assert!(ui_surface.camera_root_nodes.contains_key(&camera_entity));
        assert!(ui_surface.root_node_data.contains_key(&root_node_entity));
        assert!(ui_surface.camera_root_nodes.contains_key(&camera_entity));
        let _root_node_data =
            get_root_node_data_exact(&ui_surface, root_node_entity, camera_entity)
                .expect("expected root node data");
        assert!(ui_surface
            .camera_root_nodes
            .get(&camera_entity)
            .unwrap()
            .contains(&root_node_entity));

        ui_surface.remove_camera_entities([camera_entity]);

        // should not affect `entity_to_taffy`
        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        // `camera_roots` and `camera_entity_to_taffy` should no longer contain entries for `camera_entity`
        assert!(!ui_surface.camera_root_nodes.contains_key(&camera_entity));

        assert!(!ui_surface.camera_root_nodes.contains_key(&camera_entity));

        // root node data should be removed
        let root_node_data = get_root_node_data_exact(&ui_surface, root_node_entity, camera_entity);
        assert_eq!(root_node_data, None);
    }

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
            .camera_root_nodes
            .get(&camera_entity)
            .unwrap()
            .contains(&root_node_entity));

        ui_surface.remove_entities([root_node_entity]);
        assert!(!ui_surface.entity_to_taffy.contains_key(&root_node_entity));
        assert!(!ui_surface
            .camera_root_nodes
            .get(&camera_entity)
            .unwrap()
            .contains(&root_node_entity));
        assert!(ui_surface
            .camera_root_nodes
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
        assert_eq!(ui_surface.taffy.parent(child_node), Some(parent_node));
    }

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
            .add_child(root_taffy_node, child_taffy)
            .unwrap();

        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter());

        assert!(
            ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .unwrap()
                .contains(&root_node_entity),
            "root node not associated with camera"
        );
        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .unwrap()
                .contains(&child_entity),
            "child of root node should not be associated with camera"
        );

        let _root_node_data =
            get_root_node_data_exact(&ui_surface, root_node_entity, camera_entity)
                .expect("expected root node data");

        assert_eq!(ui_surface.taffy.parent(child_taffy), Some(root_taffy_node));
        let root_taffy_children = ui_surface.taffy.children(root_taffy_node).unwrap();
        assert!(
            root_taffy_children.contains(&child_taffy),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node),
            1,
            "expected root node child count to be 1"
        );

        // clear camera's root nodes
        ui_surface.set_camera_children(camera_entity, Vec::<Entity>::new().into_iter());

        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .unwrap()
                .contains(&root_node_entity),
            "root node should have been unassociated with camera"
        );
        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .unwrap()
                .contains(&child_entity),
            "child of root node should not be associated with camera"
        );

        let root_taffy_children = ui_surface.taffy.children(root_taffy_node).unwrap();
        assert!(
            root_taffy_children.contains(&child_taffy),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node),
            1,
            "expected root node child count to be 1"
        );

        // re-associate root node with camera
        ui_surface.set_camera_children(camera_entity, vec![root_node_entity].into_iter());

        assert!(
            ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .unwrap()
                .contains(&root_node_entity),
            "root node should have been re-associated with camera"
        );
        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .unwrap()
                .contains(&child_entity),
            "child of root node should not be associated with camera"
        );

        let child_taffy = ui_surface.entity_to_taffy.get(&child_entity).unwrap();
        let root_taffy_children = ui_surface.taffy.children(root_taffy_node).unwrap();
        assert!(
            root_taffy_children.contains(child_taffy),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node),
            1,
            "expected root node child count to be 1"
        );
    }
}
