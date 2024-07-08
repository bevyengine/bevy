use std::fmt;

use taffy::TaffyTree;

use bevy_ecs::entity::{Entity, EntityHashMap};
use bevy_ecs::prelude::Resource;
use bevy_hierarchy::Children;
use bevy_math::UVec2;
use bevy_utils::default;
use bevy_utils::tracing::warn;

use crate::layout::convert;
use crate::{LayoutContext, LayoutError, Measure, NodeMeasure, Style};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootNodePair {
    // The implicit "viewport" node created by Bevy
    pub(super) implicit_viewport_node: taffy::NodeId,
    // The root (parentless) node specified by the user
    pub(super) user_root_node: taffy::NodeId,
}

#[derive(Resource)]
pub struct UiSurface {
    pub(super) entity_to_taffy: EntityHashMap<taffy::NodeId>,
    pub(super) camera_entity_to_taffy: EntityHashMap<EntityHashMap<taffy::NodeId>>,
    pub(super) camera_roots: EntityHashMap<Vec<RootNodePair>>,
    pub(super) taffy: TaffyTree<NodeMeasure>,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<EntityHashMap<taffy::NodeId>>();
    _assert_send_sync::<TaffyTree<NodeMeasure>>();
    _assert_send_sync::<UiSurface>();
}

impl fmt::Debug for UiSurface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UiSurface")
            .field("entity_to_taffy", &self.entity_to_taffy)
            .field("camera_roots", &self.camera_roots)
            .finish()
    }
}

impl Default for UiSurface {
    fn default() -> Self {
        let mut taffy: TaffyTree<NodeMeasure> = TaffyTree::new();
        taffy.disable_rounding();
        Self {
            entity_to_taffy: Default::default(),
            camera_entity_to_taffy: Default::default(),
            camera_roots: Default::default(),
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
    pub fn update_children(&mut self, entity: Entity, children: &Children) {
        let mut taffy_children = Vec::with_capacity(children.len());
        for child in children {
            if let Some(taffy_node) = self.entity_to_taffy.get(child) {
                taffy_children.push(*taffy_node);
            } else {
                warn!(
                    "Unstyled child `{child}` in a UI entity hierarchy. You are using an entity \
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

    /// Set the ui node entities without a [`bevy_hierarchy::Parent`] as children to the root node in the taffy layout.
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
            let node = *self.entity_to_taffy.get(&entity).unwrap();
            let root_node = existing_roots
                .iter()
                .find(|n| n.user_root_node == node)
                .cloned()
                .unwrap_or_else(|| {
                    if let Some(previous_parent) = self.taffy.parent(node) {
                        // remove the root node from the previous implicit node's children
                        self.taffy.remove_child(previous_parent, node).unwrap();
                    }

                    let viewport_node = *camera_root_node_map
                        .entry(entity)
                        .or_insert_with(|| self.taffy.new_leaf(viewport_style.clone()).unwrap());
                    self.taffy.add_child(viewport_node, node).unwrap();

                    RootNodePair {
                        implicit_viewport_node: viewport_node,
                        user_root_node: node,
                    }
                });
            new_roots.push(root_node);
        }

        self.camera_roots.insert(camera_id, new_roots);
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_camera_layout(&mut self, camera: Entity, render_target_resolution: UVec2) {
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

    /// Removes each camera entity from the internal map and then removes their associated node from taffy
    pub fn remove_camera_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            if let Some(camera_root_node_map) = self.camera_entity_to_taffy.remove(&entity) {
                for (_, node) in camera_root_node_map.iter() {
                    self.taffy.remove(*node).unwrap();
                }
            }
        }
    }

    /// Removes each entity from the internal map and then removes their associated node from taffy
    pub fn remove_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            if let Some(node) = self.entity_to_taffy.remove(&entity) {
                self.taffy.remove(node).unwrap();
            }
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
}
