use std::fmt;

use taffy::{NodeId, TaffyTree, TraversePartialTree};

use bevy_ecs::entity::{Entity, EntityHashMap, EntityHashSet};
use bevy_ecs::prelude::Resource;
use bevy_math::UVec2;
use bevy_utils::tracing::warn;
use bevy_utils::{default, HashMap};

use crate::layout::convert;
use crate::{LayoutContext, LayoutError, Measure, NodeMeasure, Style};

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
    /// Maps `Entity` to its corresponding taffy node
    ///
    /// Maintains an entry for each root ui node (parentless), and any of its children
    ///
    /// (does not include the `implicit_viewport_node`)
    entity_to_taffy: EntityHashMap<taffy::NodeId>,
    /// Maps root ui node (parentless) `Entity` to its corresponding `RootNodeData`
    root_node_data: EntityHashMap<RootNodeData>,
    /// Maps camera `Entity` to an associated `EntityHashSet` of root nodes (parentless)
    camera_root_nodes: EntityHashMap<EntityHashSet>,
    /// Manages the UI Node Tree
    taffy: TaffyTree<NodeMeasure>,
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
        let mut taffy: TaffyTree<NodeMeasure> = TaffyTree::new();
        taffy.disable_rounding();
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
        style: &Style,
        mut new_node_context: Option<NodeMeasure>,
    ) {
        let taffy = &mut self.taffy;

        let mut added = false;
        let taffy_node_id = *self.entity_to_taffy.entry(entity).or_insert_with(|| {
            added = true;
            if let Some(measure) = new_node_context.take() {
                taffy
                    .new_leaf_with_context(
                        convert::from_style(layout_context, style, true),
                        measure,
                    )
                    .unwrap()
            } else {
                taffy
                    .new_leaf(convert::from_style(layout_context, style, false))
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
                    convert::from_style(layout_context, style, has_measure),
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
    pub fn update_children(&mut self, entity: Entity, children: &[Entity]) {
        let mut taffy_children = Vec::with_capacity(children.len());
        for child in children {
            if let Some(taffy_node) = self.entity_to_taffy.get(child) {
                taffy_children.push(*taffy_node);
            } else {
                warn!(
                    "Unstyled child in a UI entity hierarchy. You are using an entity \
without UI components as a child of an entity with UI components, results may be unexpected."
                );
            }
        }

        let taffy_node = self.entity_to_taffy.get(&entity).unwrap();
        self.taffy
            .set_children(*taffy_node, &taffy_children)
            .unwrap();
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
    fn mark_root_node_as_orphaned(&mut self, root_node_entity: &Entity) {
        self.replace_camera_association(*root_node_entity, None);
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
        root_node_entity: &Entity,
        camera_entity: &Entity,
    ) -> &mut RootNodeData {
        let user_root_node = *self.entity_to_taffy.get(root_node_entity).expect("create_or_update_root_node_data called before ui_root_node_entity was added to taffy tree or was previously removed");
        let ui_root_node_entity = *root_node_entity;
        let camera_entity = *camera_entity;

        let mut added = false;

        // creates mutable borrow on self that lives as long as the result
        let _ = self
            .root_node_data
            .entry(ui_root_node_entity)
            .or_insert_with(|| {
                added = true;

                self.camera_root_nodes
                    .entry(camera_entity)
                    .or_default()
                    .insert(ui_root_node_entity);

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
            self.replace_camera_association(ui_root_node_entity, Some(camera_entity));
        }

        self.root_node_data
            .get_mut(root_node_entity)
            .unwrap_or_else(|| unreachable!())
    }

    /// Set the ui node entities without a [`bevy_hierarchy::Parent`] as children to the root node in the taffy layout.
    pub fn set_camera_children(
        &mut self,
        camera_entity: Entity,
        children: impl Iterator<Item = Entity>,
    ) {
        let removed_children = self.camera_root_nodes.entry(camera_entity).or_default();
        let mut removed_children = removed_children.clone();

        for ui_entity in children {
            // creates mutable borrow on self that lives as long as the result
            let _ = self.create_or_update_root_node_data(&ui_entity, &camera_entity);

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

        for orphan in removed_children.iter() {
            self.mark_root_node_as_orphaned(orphan);
        }
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_camera_layout(
        &mut self,
        camera_entity: &Entity,
        render_target_resolution: UVec2,
    ) {
        let Some(root_nodes) = self.camera_root_nodes.get(camera_entity) else {
            return;
        };
        for &root_node_entity in root_nodes.iter() {
            let available_space = taffy::geometry::Size {
                width: taffy::style::AvailableSpace::Definite(render_target_resolution.x as f32),
                height: taffy::style::AvailableSpace::Definite(render_target_resolution.y as f32),
            };

            let root_node_data = self
                .root_node_data
                .get(&root_node_entity)
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
                                let size = ctx.measure(
                                    known_dimensions.width,
                                    known_dimensions.height,
                                    available_space.width,
                                    available_space.height,
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
    pub(super) fn remove_camera(&mut self, camera_entity: &Entity) {
        if let Some(root_node_entities) = self.camera_root_nodes.remove(camera_entity) {
            for root_node_entity in root_node_entities {
                self.remove_root_node_viewport(&root_node_entity);
            }
        };
    }

    /// Disassociates the root node from the assigned camera (if any) and removes the viewport node from taffy
    /// Removes entry in `root_node_data`
    pub(super) fn remove_root_node_viewport(&mut self, root_node_entity: &Entity) {
        self.mark_root_node_as_orphaned(root_node_entity);
        if let Some(removed) = self.root_node_data.remove(root_node_entity) {
            self.taffy.remove(removed.implicit_viewport_node).unwrap();
        }
    }

    /// Removes the ui node from the taffy tree, and if it's a root node it also calls `remove_root_node_viewport`
    pub(super) fn remove_ui_node(&mut self, ui_node_entity: &Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.remove(ui_node_entity) {
            self.taffy.remove(taffy_node).unwrap();
        }
        // remove root node entry if this is a root node
        if self.root_node_data.contains_key(ui_node_entity) {
            self.remove_root_node_viewport(ui_node_entity);
        }
    }

    /// Removes specified camera entities by disassociating them from their associated `implicit_viewport_node`
    /// in the internal map, and subsequently removes the `implicit_viewport_node`
    /// from the `taffy` layout engine for each.
    pub fn remove_camera_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            self.remove_camera(&entity);
        }
    }

    /// Removes the specified entities from the internal map while
    /// removing their `implicit_viewport_node` from taffy,
    /// and then subsequently removes their entry from `entity_to_taffy` and associated node from taffy
    pub fn remove_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            self.remove_ui_node(&entity);
        }
    }

    /// Get the layout geometry for the taffy node corresponding to the ui node [`Entity`].
    /// Does not compute the layout geometry, `compute_window_layouts` should be run before using this function.
    pub fn get_layout(&self, entity: Entity) -> Result<&taffy::Layout, LayoutError> {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy
                .layout(*taffy_node)
                .map_err(LayoutError::TaffyError)
        } else {
            warn!(
                "Styled child in a non-UI entity hierarchy. You are using an entity \
with UI components as a child of an entity without UI components, results may be unexpected."
            );
            Err(LayoutError::InvalidHierarchy)
        }
    }

    /// Returns a debug representation of the computed layout of the UI layout tree for each camera.
    pub fn ui_layout_tree_debug_string(&self) -> Result<String, fmt::Error> {
        use std::fmt::Write;
        let mut output = String::new();
        let taffy_to_entity: HashMap<NodeId, Entity> = self
            .entity_to_taffy
            .iter()
            .map(|(&ui_entity, &taffy_node)| (taffy_node, ui_entity))
            .collect::<HashMap<NodeId, Entity>>();
        for (&camera_entity, root_node_set) in self.camera_root_nodes.iter() {
            writeln!(output, "Layout tree for camera entity: {camera_entity}")?;
            for &root_node_entity in root_node_set.iter() {
                let Some(implicit_viewport_node) = self
                    .root_node_data
                    .get(&root_node_entity)
                    .map(|rnd| rnd.implicit_viewport_node)
                else {
                    continue;
                };
                self.write_node(
                    &taffy_to_entity,
                    camera_entity,
                    implicit_viewport_node,
                    false,
                    String::new(),
                    &mut output,
                )?;
            }
        }
        Ok(output)
    }

    /// Recursively navigates the layout tree writing each node's information to the output/acc.
    fn write_node(
        &self,
        taffy_to_entity: &HashMap<NodeId, Entity>,
        entity: Entity,
        node: NodeId,
        has_sibling: bool,
        lines_string: String,
        acc: &mut String,
    ) -> fmt::Result {
        use std::fmt::Write;
        let tree = &self.taffy;
        let layout = tree.layout(node).unwrap();
        let style = tree.style(node).unwrap();

        let num_children = tree.child_count(node);

        let display_variant = match (num_children, style.display) {
            (_, taffy::style::Display::None) => "NONE",
            (0, _) => "LEAF",
            (_, taffy::style::Display::Flex) => "FLEX",
            (_, taffy::style::Display::Grid) => "GRID",
            (_, taffy::style::Display::Block) => "BLOCK",
        };

        let fork_string = if has_sibling {
            "├── "
        } else {
            "└── "
        };
        writeln!(
            acc,
            "{lines}{fork} {display} [x: {x:<4} y: {y:<4} width: {width:<4} height: {height:<4}] ({entity:?}) {measured}",
            lines = lines_string,
            fork = fork_string,
            display = display_variant,
            x = layout.location.x,
            y = layout.location.y,
            width = layout.size.width,
            height = layout.size.height,
            measured = if tree.get_node_context(node).is_some() { "measured" } else { "" }
        )?;
        let bar = if has_sibling { "│   " } else { "    " };
        let new_string = lines_string + bar;

        // Recurse into children
        for (index, child_node) in tree.children(node).unwrap().iter().enumerate() {
            let has_sibling = index < num_children - 1;
            let child_entity = taffy_to_entity.get(child_node).unwrap();
            self.write_node(
                taffy_to_entity,
                *child_entity,
                *child_node,
                has_sibling,
                new_string.clone(),
                acc,
            )?;
        }

        Ok(())
    }
}

// Expose readonly accessors for tests in mod
#[cfg(test)]
impl UiSurface {
    pub(super) fn entity_to_taffy(&self) -> &EntityHashMap<taffy::NodeId> {
        &self.entity_to_taffy
    }
    pub(super) fn root_node_data(&self) -> &EntityHashMap<RootNodeData> {
        &self.root_node_data
    }
    pub(super) fn camera_root_nodes(&self) -> &EntityHashMap<EntityHashSet> {
        &self.camera_root_nodes
    }
    pub(super) fn taffy(&self) -> &TaffyTree<NodeMeasure> {
        &self.taffy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentSize, FixedMeasure, Val};
    use bevy_math::Vec2;
    use taffy::TraversePartialTree;

    const TEST_LAYOUT_CONTEXT: LayoutContext = LayoutContext {
        scale_factor: 1.0,
        physical_size: Vec2::ONE,
        min_size: 0.0,
        max_size: 1.0,
    };

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
        let style = Style::default();

        // standard upsert
        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);

        // should be inserted into taffy
        assert_eq!(ui_surface.taffy.total_node_count(), 1);
        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        // test duplicate insert 1
        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);

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
        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);

        // node count should not have increased
        assert_eq!(ui_surface.taffy.total_node_count(), 2);

        // root node data should be unaffected
        let _root_node_data =
            get_root_node_data_exact(&ui_surface, root_node_entity, camera_entity)
                .expect("expected root node data");
        assert!(has_valid_root_node_data(&ui_surface, &root_node_entity));
    }

    #[test]
    fn test_get_root_node_data_exact() {
        /// Attempts to find the camera entity that holds a reference to the given root node entity
        fn get_associated_camera_entity(
            ui_surface: &UiSurface,
            root_node_entity: Entity,
        ) -> Option<Entity> {
            get_root_node_data(ui_surface, root_node_entity)?.camera_entity
        }

        /// Attempts to find the root node data corresponding to the given root node entity
        fn get_root_node_data(
            ui_surface: &UiSurface,
            root_node_entity: Entity,
        ) -> Option<&RootNodeData> {
            ui_surface.root_node_data.get(&root_node_entity)
        }

        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);

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
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);

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
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);

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
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);
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
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);
        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, child_entity, &style, None);

        ui_surface.update_children(root_node_entity, &[child_entity]);

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
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);
        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, child_entity, &style, None);

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

    #[test]
    fn test_compute_camera_layout() {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);

        ui_surface.compute_camera_layout(&camera_entity, UVec2::new(800, 600));

        let taffy_node = ui_surface.entity_to_taffy.get(&root_node_entity).unwrap();
        assert!(ui_surface.taffy.layout(*taffy_node).is_ok());
    }

    #[test]
    fn test_ui_layout_tree_debug_string() {
        let mut ui_surface = UiSurface::default();

        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let child_entity = Entity::from_raw(2);
        let style = Style::default();

        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, root_node_entity, &style, None);
        ui_surface.upsert_node(&TEST_LAYOUT_CONTEXT, child_entity, &style, None);

        let root_taffy_node = *ui_surface.entity_to_taffy.get(&root_node_entity).unwrap();
        let child_taffy = *ui_surface.entity_to_taffy.get(&child_entity).unwrap();
        // set up the relationship manually
        ui_surface
            .taffy
            .add_child(root_taffy_node, child_taffy)
            .unwrap();
        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter());

        let camera_entity2 = Entity::from_raw(3);
        let root_node_entity2 = Entity::from_raw(4);
        let style = Style {
            top: Val::Px(1.),
            left: Val::Px(1.),
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        };
        ui_surface.upsert_node(
            &TEST_LAYOUT_CONTEXT,
            root_node_entity2,
            &style,
            Some(NodeMeasure::Fixed(FixedMeasure { size: Vec2::ONE })),
        );
        ui_surface.set_camera_children(camera_entity2, [root_node_entity2].into_iter());

        ui_surface.compute_camera_layout(&camera_entity, UVec2::ONE);
        ui_surface.compute_camera_layout(&camera_entity2, UVec2::ONE);

        let debug_string = ui_surface
            .ui_layout_tree_debug_string()
            .expect("expected debug string");
        let lines = debug_string.lines().collect::<Vec<_>>();

        assert!(lines[0].starts_with("Layout tree for camera entity: 0v1|"));
        assert_eq!(lines[1], "└──  GRID [x: 0    y: 0    width: 1    height: 1   ] (Entity { index: 0, generation: 1 }) ");
        assert_eq!(lines[2], "    └──  FLEX [x: 0    y: 0    width: 0    height: 0   ] (Entity { index: 1, generation: 1 }) ");
        assert_eq!(lines[3], "        └──  LEAF [x: 0    y: 0    width: 0    height: 0   ] (Entity { index: 2, generation: 1 }) ");
        assert!(lines[4].starts_with("Layout tree for camera entity: 3v1|"));
        assert_eq!(lines[5], "└──  GRID [x: 0    y: 0    width: 1    height: 1   ] (Entity { index: 3, generation: 1 }) ");
        assert_eq!(lines[6], "    └──  LEAF [x: 1    y: 1    width: 1    height: 1   ] (Entity { index: 4, generation: 1 }) measured");
    }
}
