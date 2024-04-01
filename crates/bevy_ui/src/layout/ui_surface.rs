use std::fmt;

use taffy::prelude::LayoutTree;
use taffy::Taffy;
use thiserror::Error;

use bevy_ecs::entity::{Entity, EntityHashMap, EntityHashSet};
use bevy_ecs::prelude::Resource;
use bevy_math::UVec2;
use bevy_utils::default;
use bevy_utils::hashbrown::hash_map::Entry;
use bevy_utils::tracing::warn;

use crate::layout::convert;
use crate::{LayoutContext, LayoutError, Style};

#[derive(Debug, Error)]
/// Error wrapper for most actions performed in `UiSurface`
pub enum UiSurfaceError {
    #[error("Invalid state: {0}")]
    /// Used whenever a creating function encounters a situation where the internal maps are not returning expected results.
    /// Otherwise, where appropriate `EntityNotFound` or `TaffyError` are likely used.
    InvalidState(&'static str),
    #[error("Entity not found in internal map: {0}")]
    /// Used for when an entity is not found in a map.
    /// But only when it's not explicitly expected to exist by the function creating the error.
    /// Otherwise, `InvalidState` is used.
    EntityNotFound(Entity),
    #[error("Taffy error: {0}")]
    /// Simple wrapper for `TaffyError`.
    TaffyError(#[from] taffy::error::TaffyError),
    #[error("Iteration error {0:?}")]
    /// Used in loops that attempt to continue despite encountering an error.
    IterationError(Vec<UiSurfaceError>),
    #[error("Error processing {0:?}")]
    /// Wrapper error usually used in conjunction with `IterationError`.
    /// Boxed tuple associates `Entity` to the error that was encountered when processing.
    ///
    // (boxed to allow self referencing)
    ProcessingEntityError(Box<(Entity, UiSurfaceError)>),
    #[error("Unreachable")]
    /// Code paths that are deemed unreachable but avoid `unreachable!` to allow caller to handle and proceed
    Unreachable,
}

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
    pub(super) implicit_viewport_node: taffy::node::Node,
}

#[derive(Resource)]
/// Manages state and hierarchy for ui entities
pub struct UiSurface {
    /// Maps `Entity` to its corresponding taffy node
    ///
    /// Maintains an entry for each root ui node (parentless), and any of its children
    ///
    /// (does not include the `implicit_viewport_node`)
    pub(super) entity_to_taffy: EntityHashMap<taffy::node::Node>,
    /// Maps root ui node (parentless) `Entity` to its corresponding `RootNodeData`
    pub(super) root_node_data: EntityHashMap<RootNodeData>,
    /// Maps camera `Entity` to an associated `EntityHashSet` of root nodes (parentless)
    pub(super) camera_root_nodes: EntityHashMap<EntityHashSet>,
    /// Manages the UI Node Tree
    pub(super) taffy: Taffy,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<EntityHashMap<taffy::node::Node>>();
    _assert_send_sync::<EntityHashMap<RootNodeData>>();
    _assert_send_sync::<EntityHashMap<EntityHashSet>>();
    _assert_send_sync::<Taffy>();
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
        let mut taffy = Taffy::new();
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
        ui_node_entity: Entity,
        style: &Style,
        context: &LayoutContext,
    ) -> Result<(), UiSurfaceError> {
        let taffy_style = convert::from_style(context, style);

        match self.entity_to_taffy.entry(ui_node_entity) {
            Entry::Occupied(entry) => {
                self.taffy.set_style(*entry.get(), taffy_style)?;
            }
            Entry::Vacant(entry) => {
                entry.insert(self.taffy.new_leaf(taffy_style)?);
            }
        }

        Ok(())
    }

    /// Disassociates the camera from all of its assigned root nodes and removes their viewport nodes
    /// Removes entry in `camera_root_nodes`
    pub(super) fn remove_camera(&mut self, camera_entity: &Entity) -> Result<(), UiSurfaceError> {
        if let Some(root_node_entities) = self.camera_root_nodes.remove(camera_entity) {
            for root_node_entity in root_node_entities {
                self.remove_root_node_viewport(&root_node_entity)?;
            }
        };

        Ok(())
    }

    /// Disassociates the root node from the assigned camera (if any) and removes the viewport node from taffy
    /// Removes entry in `root_node_data`
    pub(super) fn remove_root_node_viewport(
        &mut self,
        root_node_entity: &Entity,
    ) -> Result<(), UiSurfaceError> {
        self.mark_root_node_as_orphaned(root_node_entity)?;
        if let Some(removed) = self.root_node_data.remove(root_node_entity) {
            self.taffy.remove(removed.implicit_viewport_node)?;
        }

        Ok(())
    }

    /// Removes the ui node from the taffy tree, and if it's a root node it also calls `remove_root_node_viewport`
    pub(super) fn remove_ui_node(&mut self, ui_node_entity: &Entity) -> Result<(), UiSurfaceError> {
        self.remove_root_node_viewport(ui_node_entity)?;
        if let Some(taffy_node) = self.entity_to_taffy.remove(ui_node_entity) {
            self.taffy.remove(taffy_node)?;
        }

        Ok(())
    }

    /// Demotes root node to a child node of the specified parent
    pub(super) fn demote_ui_node(
        &mut self,
        target_entity: &Entity,
        parent_entity: &Entity,
    ) -> Result<(), UiSurfaceError> {
        // remove camera association
        self.mark_root_node_as_orphaned(target_entity)?;

        if let Some(root_node_data) = self.root_node_data.remove(target_entity) {
            self.taffy.remove(root_node_data.implicit_viewport_node)?;
            let parent_taffy =
                self.entity_to_taffy
                    .get(parent_entity)
                    .ok_or(UiSurfaceError::InvalidState(
                        "entity missing in entity_to_taffy",
                    ))?;
            let child_taffy =
                self.entity_to_taffy
                    .get(target_entity)
                    .ok_or(UiSurfaceError::InvalidState(
                        "entity missing in entity_to_taffy",
                    ))?;
            self.taffy.add_child(*parent_taffy, *child_taffy)?;
        }
        Ok(())
    }

    #[cfg(test)]
    /// Converts ui node to root node
    /// Should only be used for testing - does not set `TargetCamera`
    pub(super) fn promote_ui_node(
        &mut self,
        target_entity: &Entity,
        camera_entity: &Entity,
    ) -> Result<(), UiSurfaceError> {
        match self.root_node_data.entry(*target_entity) {
            Entry::Occupied(_) => {}
            Entry::Vacant(entry) => {
                let user_root_node = *self.entity_to_taffy.get(target_entity).ok_or(
                    UiSurfaceError::InvalidState("entity missing in entity_to_taffy"),
                )?;

                // clear the parent - new_with_children doesn't seem to notify the old parent its children have changed
                if let Some(parent) = self.taffy.parent(user_root_node) {
                    self.taffy.remove_child(parent, user_root_node)?;
                }

                let implicit_viewport_node = self
                    .taffy
                    .new_with_children(default_viewport_style(), &[user_root_node])?;

                entry.insert(RootNodeData {
                    camera_entity: None,
                    implicit_viewport_node,
                });
            }
        }

        self.replace_camera_association(*target_entity, Some(*camera_entity))?;

        Ok(())
    }

    /// Update the `MeasureFunc` of the taffy node corresponding to the given [`Entity`] if the node exists.
    pub fn try_update_measure(
        &mut self,
        entity: Entity,
        measure_func: taffy::node::MeasureFunc,
    ) -> Result<(), UiSurfaceError> {
        let taffy_node = self
            .entity_to_taffy
            .get(&entity)
            .ok_or(UiSurfaceError::EntityNotFound(entity))?;

        self.taffy
            .set_measure(*taffy_node, Some(measure_func))
            .map_err(UiSurfaceError::TaffyError)?;

        Ok(())
    }

    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(
        &mut self,
        entity: Entity,
        children: &[Entity],
    ) -> Result<(), UiSurfaceError> {
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

        let taffy_node = self
            .entity_to_taffy
            .get(&entity)
            .ok_or(UiSurfaceError::InvalidState(
                "entity missing in entity_to_taffy",
            ))?;

        self.taffy.set_children(*taffy_node, &taffy_children)?;

        Ok(())
    }

    /// Removes children from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_children(&mut self, entity: Entity) -> Result<(), UiSurfaceError> {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_children(*taffy_node, &[])?;
        }

        Ok(())
    }

    /// Removes the measure from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_measure(&mut self, entity: Entity) -> Result<(), UiSurfaceError> {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_measure(*taffy_node, None)?;
        }

        Ok(())
    }

    /// Removes camera association to root node
    /// Shorthand for calling `replace_camera_association(root_node_entity, None)`
    fn mark_root_node_as_orphaned(
        &mut self,
        root_node_entity: &Entity,
    ) -> Result<(), UiSurfaceError> {
        self.replace_camera_association(*root_node_entity, None)?;

        Ok(())
    }

    /// Reassigns or removes a root node's associated camera entity
    /// `Some(camera_entity)` - Updates camera association to root node
    /// `None` - Removes camera association to root node
    /// Does not check to see if they are the same before performing operations
    fn replace_camera_association(
        &mut self,
        root_node_entity: Entity,
        new_camera_entity_option: Option<Entity>,
    ) -> Result<(), UiSurfaceError> {
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

        Ok(())
    }

    /// Creates or updates a root node
    fn create_or_update_root_node_data(
        &mut self,
        root_node_entity: &Entity,
        camera_entity: &Entity,
    ) -> Result<&RootNodeData, UiSurfaceError> {
        let user_root_node = *self.entity_to_taffy
            .get(root_node_entity)
            .ok_or(UiSurfaceError::InvalidState("create_or_update_root_node_data called before ui_root_node_entity was added to taffy tree or was previously removed"))?;
        let root_node_entity = *root_node_entity;
        let camera_entity = *camera_entity;

        // creates mutable borrow on self that lives as long as the result
        match self.root_node_data.entry(root_node_entity) {
            Entry::Occupied(_) => {
                self.replace_camera_association(root_node_entity, Some(camera_entity))?;
            }
            Entry::Vacant(entry) => {
                self.camera_root_nodes
                    .entry(camera_entity)
                    .or_default()
                    .insert(root_node_entity);

                let implicit_viewport_node = self.taffy.new_leaf(default_viewport_style())?;

                self.taffy
                    .add_child(implicit_viewport_node, user_root_node)?;

                entry.insert(RootNodeData {
                    camera_entity: Some(camera_entity),
                    implicit_viewport_node,
                });
            }
        };

        Ok(self
            .root_node_data
            .get_mut(&root_node_entity)
            .ok_or(UiSurfaceError::Unreachable)?)
    }

    /// Set the ui node entities without a [`Parent`] as children to the root node in the taffy layout.
    pub fn set_camera_children(
        &mut self,
        camera_entity: Entity,
        children: impl Iterator<Item = Entity>,
    ) -> Result<(), UiSurfaceError> {
        let removed_children = self.camera_root_nodes.entry(camera_entity).or_default();
        let mut removed_children = removed_children.clone();

        let mut update =
            |camera_entity: Entity, child_entity: Entity| -> Result<(), UiSurfaceError> {
                // creates mutable borrow on self that lives as long as the result
                let _ = self.create_or_update_root_node_data(&child_entity, &camera_entity)?;

                // drop the mutable borrow on self by re-fetching
                let root_node_data = self
                    .root_node_data
                    .get(&child_entity)
                    .ok_or(UiSurfaceError::Unreachable)?;

                // fix taffy relationships
                {
                    let taffy_node = *self
                        .entity_to_taffy
                        .get(&child_entity)
                        .ok_or(UiSurfaceError::Unreachable)?;

                    if let Some(parent) = self.taffy.parent(taffy_node) {
                        self.taffy.remove_child(parent, taffy_node)?;
                    }

                    self.taffy
                        .add_child(root_node_data.implicit_viewport_node, taffy_node)?;
                }

                Ok(())
            };

        let mut errors = vec![];

        for ui_entity in children {
            match update(camera_entity, ui_entity) {
                Ok(_) => {}
                Err(err) => {
                    errors.push(UiSurfaceError::ProcessingEntityError(Box::new((
                        ui_entity, err,
                    ))));
                }
            }

            removed_children.remove(&ui_entity);
        }

        for orphan in removed_children.iter() {
            self.mark_root_node_as_orphaned(orphan)?;
        }

        if !errors.is_empty() {
            return Err(UiSurfaceError::IterationError(errors));
        }

        Ok(())
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_camera_layout(
        &mut self,
        camera_entity: &Entity,
        render_target_resolution: UVec2,
    ) -> Result<(), UiSurfaceError> {
        let root_nodes = self
            .camera_root_nodes
            .get(camera_entity)
            .ok_or(UiSurfaceError::EntityNotFound(*camera_entity))?;

        let mut update = |root_node_entity: Entity| -> Result<(), UiSurfaceError> {
            let available_space = taffy::geometry::Size {
                width: taffy::style::AvailableSpace::Definite(render_target_resolution.x as f32),
                height: taffy::style::AvailableSpace::Definite(render_target_resolution.y as f32),
            };

            let root_node_data =
                self.root_node_data
                    .get(&root_node_entity)
                    .ok_or(UiSurfaceError::InvalidState(
                        "root_node_data missing entity",
                    ))?;

            if root_node_data.camera_entity.is_none() {
                return Err(UiSurfaceError::InvalidState(
                    "root node not associated with any camera",
                ));
            }

            self.taffy
                .compute_layout(root_node_data.implicit_viewport_node, available_space)?;

            Ok(())
        };

        let mut errors = vec![];

        for &root_node_entity in root_nodes.iter() {
            match update(root_node_entity) {
                Ok(_) => {}
                Err(err) => {
                    errors.push(UiSurfaceError::ProcessingEntityError(Box::new((
                        root_node_entity,
                        err,
                    ))));
                }
            }
        }

        if !errors.is_empty() {
            return Err(UiSurfaceError::IterationError(errors));
        }

        Ok(())
    }

    /// Removes specified camera entities by disassociating them from their associated `implicit_viewport_node`
    /// in the internal map, and subsequently removes the `implicit_viewport_node`
    /// from the `taffy` layout engine for each.
    pub fn remove_camera_entities(
        &mut self,
        entities: impl IntoIterator<Item = Entity>,
    ) -> Result<(), UiSurfaceError> {
        let mut errors = vec![];

        for entity in entities {
            let res = self.remove_camera(&entity);
            if let Err(err) = res {
                errors.push(UiSurfaceError::ProcessingEntityError(Box::new((
                    entity, err,
                ))));
            }
        }

        if !errors.is_empty() {
            return Err(UiSurfaceError::IterationError(errors));
        }

        Ok(())
    }

    /// Removes the specified entities from the internal map while
    /// removing their `implicit_viewport_node` from taffy,
    /// and then subsequently removes their entry from `entity_to_taffy` and associated node from taffy
    pub fn remove_entities(
        &mut self,
        entities: impl IntoIterator<Item = Entity>,
    ) -> Result<(), UiSurfaceError> {
        let mut errors = vec![];

        for entity in entities {
            let res = self.remove_ui_node(&entity);
            if let Err(err) = res {
                errors.push(UiSurfaceError::ProcessingEntityError(Box::new((
                    entity, err,
                ))));
            }
        }

        if !errors.is_empty() {
            return Err(UiSurfaceError::IterationError(errors));
        }

        Ok(())
    }

    /// Get the layout geometry for the taffy node corresponding to the ui node [`Entity`].
    /// Does not compute the layout geometry, `compute_window_layouts` should be run before using this function.
    pub fn get_layout(&self, entity: Entity) -> Result<&taffy::layout::Layout, LayoutError> {
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

    #[cfg(test)]
    /// Tries to get the associated camera entity from root node data
    fn get_associated_camera_entity(&self, root_node_entity: Entity) -> Option<Entity> {
        self.get_root_node_data(root_node_entity)?.camera_entity
    }

    #[cfg(test)]
    /// Tries to get the root node data for a given root node entity
    fn get_root_node_data(&self, root_node_entity: Entity) -> Option<&RootNodeData> {
        self.root_node_data.get(&root_node_entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentSize, FixedMeasure};
    use bevy_math::Vec2;
    #[test]
    fn test_initialization() {
        let ui_surface = UiSurface::default();
        assert!(ui_surface.entity_to_taffy.is_empty());
        assert!(ui_surface.root_node_data.is_empty());
        assert!(ui_surface.camera_root_nodes.is_empty());
        assert_eq!(ui_surface.taffy.total_node_count(), 0);
    }

    const TEST_LAYOUT_CONTEXT: LayoutContext = LayoutContext {
        scale_factor: 1.0,
        physical_size: Vec2::ONE,
        min_size: 0.0,
        max_size: 1.0,
    };

    trait HasValidRootNodeData {
        fn has_valid_root_node_data(&self, root_node_entity: &Entity) -> bool;
    }

    impl HasValidRootNodeData for UiSurface {
        fn has_valid_root_node_data(&self, root_node_entity: &Entity) -> bool {
            let Some(&taffy_node) = self.entity_to_taffy.get(root_node_entity) else {
                return false;
            };
            let Some(root_node_data) = self.root_node_data.get(root_node_entity) else {
                return false;
            };
            self.taffy.parent(taffy_node) == Some(root_node_data.implicit_viewport_node)
        }
    }

    #[test]
    fn test_upsert() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let style = Style::default();

        // standard upsert
        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        // should be inserted into taffy
        assert_eq!(ui_surface.taffy.total_node_count(), 1);
        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        // test duplicate insert 1
        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        // node count should not have increased
        assert_eq!(ui_surface.taffy.total_node_count(), 1);

        // assign root node to camera
        ui_surface.set_camera_children(camera_entity, vec![root_node_entity].into_iter())?;

        // each root node will create 2 taffy nodes
        assert_eq!(ui_surface.taffy.total_node_count(), 2);

        // root node data should now exist
        assert!(ui_surface.has_valid_root_node_data(&root_node_entity));

        // test duplicate insert 2
        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        // node count should not have increased
        assert_eq!(ui_surface.taffy.total_node_count(), 2);

        // root node data should be unaffected
        assert!(ui_surface.has_valid_root_node_data(&root_node_entity));

        Ok(())
    }

    #[test]
    fn test_remove_camera_entities() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let style = Style::default();

        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        // assign root node to camera
        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter())?;

        assert!(ui_surface.camera_root_nodes.contains_key(&camera_entity));
        assert!(ui_surface.root_node_data.contains_key(&root_node_entity));
        let _root_node_data = ui_surface
            .get_root_node_data(root_node_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        assert!(ui_surface
            .camera_root_nodes
            .get(&camera_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?
            .contains(&root_node_entity));

        ui_surface.remove_camera_entities([camera_entity])?;

        // should not affect `entity_to_taffy`
        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        // `camera_roots` and `camera_entity_to_taffy` should no longer contain entries for `camera_entity`
        assert!(!ui_surface.camera_root_nodes.contains_key(&camera_entity));

        assert!(!ui_surface.camera_root_nodes.contains_key(&camera_entity));

        // root node data should be removed
        let root_node_data = ui_surface.get_root_node_data(root_node_entity);
        assert_eq!(root_node_data, None);

        Ok(())
    }

    #[test]
    fn test_remove_entities() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let style = Style::default();

        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter())?;

        assert!(ui_surface.entity_to_taffy.contains_key(&root_node_entity));
        assert!(ui_surface
            .camera_root_nodes
            .get(&camera_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?
            .contains(&root_node_entity));
        let _root_node_data = ui_surface
            .get_root_node_data(root_node_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;

        ui_surface.remove_entities([root_node_entity])?;
        assert!(!ui_surface.entity_to_taffy.contains_key(&root_node_entity));

        assert!(!ui_surface
            .camera_root_nodes
            .get(&camera_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?
            .contains(&root_node_entity));
        assert!(ui_surface
            .camera_root_nodes
            .get(&camera_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?
            .is_empty());

        Ok(())
    }

    #[test]
    fn test_try_update_measure() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let root_node_entity = Entity::from_raw(1);
        let style = Style::default();

        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;
        let mut content_size = ContentSize::default();
        content_size.set(FixedMeasure { size: Vec2::ONE });
        let measure_func = content_size
            .measure_func
            .take()
            .ok_or(UiSurfaceError::InvalidState(""))?;
        ui_surface.try_update_measure(root_node_entity, measure_func)?;

        Ok(())
    }

    #[test]
    fn test_update_children() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let root_node_entity = Entity::from_raw(1);
        let child_entity = Entity::from_raw(2);
        let style = Style::default();

        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;
        ui_surface.upsert_node(child_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        ui_surface.update_children(root_node_entity, &[child_entity])?;

        let parent_node = *ui_surface
            .entity_to_taffy
            .get(&root_node_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        let child_node = *ui_surface
            .entity_to_taffy
            .get(&child_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        assert_eq!(ui_surface.taffy.parent(child_node), Some(parent_node));

        Ok(())
    }

    #[test]
    fn test_set_camera_children() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let child_entity = Entity::from_raw(2);
        let style = Style::default();

        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;
        ui_surface.upsert_node(child_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        let root_taffy_node = *ui_surface
            .entity_to_taffy
            .get(&root_node_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        let child_taffy = *ui_surface
            .entity_to_taffy
            .get(&child_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;

        // set up the relationship manually
        ui_surface.taffy.add_child(root_taffy_node, child_taffy)?;

        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter())?;

        assert!(
            ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .ok_or(UiSurfaceError::InvalidState(""))?
                .contains(&root_node_entity),
            "root node not associated with camera"
        );
        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .ok_or(UiSurfaceError::InvalidState(""))?
                .contains(&child_entity),
            "child of root node should not be associated with camera"
        );

        let _root_node_data = ui_surface
            .get_root_node_data(root_node_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;

        assert_eq!(ui_surface.taffy.parent(child_taffy), Some(root_taffy_node));
        let root_taffy_children = ui_surface.taffy.children(root_taffy_node)?;
        assert!(
            root_taffy_children.contains(&child_taffy),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node)?,
            1,
            "expected root node child count to be 1"
        );

        // clear camera's root nodes
        ui_surface.set_camera_children(camera_entity, Vec::<Entity>::new().into_iter())?;

        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .ok_or(UiSurfaceError::InvalidState(""))?
                .contains(&root_node_entity),
            "root node should have been unassociated with camera"
        );
        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .ok_or(UiSurfaceError::InvalidState(""))?
                .contains(&child_entity),
            "child of root node should not be associated with camera"
        );

        let root_taffy_children = ui_surface.taffy.children(root_taffy_node)?;
        assert!(
            root_taffy_children.contains(&child_taffy),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node)?,
            1,
            "expected root node child count to be 1"
        );

        // re-associate root node with camera
        ui_surface.set_camera_children(camera_entity, vec![root_node_entity].into_iter())?;

        assert!(
            ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .ok_or(UiSurfaceError::InvalidState(""))?
                .contains(&root_node_entity),
            "root node should have been re-associated with camera"
        );
        assert!(
            !ui_surface
                .camera_root_nodes
                .get(&camera_entity)
                .ok_or(UiSurfaceError::InvalidState(""))?
                .contains(&child_entity),
            "child of root node should not be associated with camera"
        );

        let child_taffy = ui_surface
            .entity_to_taffy
            .get(&child_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        let root_taffy_children = ui_surface.taffy.children(root_taffy_node)?;
        assert!(
            root_taffy_children.contains(child_taffy),
            "root node is not a parent of child node"
        );
        assert_eq!(
            ui_surface.taffy.child_count(root_taffy_node)?,
            1,
            "expected root node child count to be 1"
        );

        Ok(())
    }

    #[test]
    fn test_compute_camera_layout() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let style = Style::default();

        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;

        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter())?;

        ui_surface.compute_camera_layout(&camera_entity, UVec2::new(800, 600))?;

        let taffy_node = ui_surface
            .entity_to_taffy
            .get(&root_node_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        ui_surface.taffy.layout(*taffy_node)?;

        Ok(())
    }

    #[test]
    fn test_promotion() -> Result<(), UiSurfaceError> {
        let mut ui_surface = UiSurface::default();
        let camera_entity = Entity::from_raw(0);
        let root_node_entity = Entity::from_raw(1);
        let child_entity = Entity::from_raw(2);
        let style = Style::default();

        ui_surface.upsert_node(root_node_entity, &style, &TEST_LAYOUT_CONTEXT)?;
        ui_surface.upsert_node(child_entity, &style, &TEST_LAYOUT_CONTEXT)?;
        ui_surface.update_children(root_node_entity, &[child_entity])?;
        assert_eq!(ui_surface.taffy.total_node_count(), 2);

        ui_surface.set_camera_children(camera_entity, [root_node_entity].into_iter())?;
        assert_eq!(ui_surface.taffy.total_node_count(), 3);

        ui_surface.promote_ui_node(&child_entity, &camera_entity)?;
        assert_eq!(ui_surface.taffy.total_node_count(), 4);
        assert_eq!(
            ui_surface.get_associated_camera_entity(child_entity),
            Some(camera_entity)
        );

        let root_node_entity_taffy = ui_surface
            .entity_to_taffy
            .get(&root_node_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        let child_entity_taffy = ui_surface
            .entity_to_taffy
            .get(&child_entity)
            .ok_or(UiSurfaceError::InvalidState(""))?;
        assert!(!ui_surface
            .taffy
            .children(*root_node_entity_taffy)?
            .contains(child_entity_taffy));

        Ok(())
    }
}
