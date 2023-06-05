mod convert;
pub mod debug;

use crate::{ContentSize, Node, Style};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::Component,
    query::{With, Without},
    removal_detection::RemovedComponents,
    system::{Query, ResMut, Resource},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_reflect::{FromReflect, Reflect};
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use std::fmt;
use taffy::{prelude::Size, style_helpers::TaffyMaxContent, Taffy};

type TaffyNode = taffy::node::Node;

/// The size and scaling information for a UI layout derived from its render target.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LayoutContext {
    /// Physical size of the target in pixels.
    pub physical_size: Vec2,
    /// Product of the target's scale factor and the camera's `UiScale`.
    pub combined_scale_factor: f64,
    /// Inverse of the target's scale factor.
    pub inverse_target_scale_factor: f64,
}

impl LayoutContext {
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    pub(crate) fn new(physical_size: Vec2, target_scale_factor: f64, ui_scale: f64) -> Self {
        let combined_scale_factor = ui_scale * target_scale_factor;
        let inverse_target_scale_factor = target_scale_factor.recip();
        Self {
            physical_size,
            combined_scale_factor,
            inverse_target_scale_factor,
        }
    }

    pub(crate) fn root_style(&self) -> taffy::style::Style {
        taffy::style::Style {
            size: taffy::geometry::Size {
                width: taffy::style::Dimension::Points(self.physical_size.x),
                height: taffy::style::Dimension::Points(self.physical_size.y),
            },
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct UiLayoutRoot {
    pub(crate) taffy_root: TaffyNode,
    pub(crate) context: LayoutContext,
    pub(crate) perform_full_update: bool,
    pub(crate) root_uinodes: Vec<Entity>,
}

impl UiLayoutRoot {
    pub(crate) fn new(taffy_root: TaffyNode, layout_context: LayoutContext) -> Self {
        Self {
            taffy_root,
            context: layout_context,
            perform_full_update: true,
            root_uinodes: vec![],
        }
    }
}

#[derive(Component, Debug, Reflect, FromReflect)]
pub struct UiTargetCamera {
    pub entity: Entity,
}

#[derive(Resource)]
pub struct UiSurface {
    pub(crate) entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    pub(crate) taffy: Taffy,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<HashMap<Entity, taffy::node::Node>>();
    _assert_send_sync::<Taffy>();
    _assert_send_sync::<UiSurface>();
}

impl fmt::Debug for UiSurface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UiSurface")
            .field("entity_to_taffy", &self.entity_to_taffy)
            .finish()
    }
}

#[derive(Resource, Default)]
pub struct UiDefaultCamera {
    // Orphaned UI nodes without a `UiTargetCamera` component are added to the default camera's associated layout.
    pub entity: Option<Entity>,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct UiCameraToRoot {
    camera_to_root: HashMap<Entity, UiLayoutRoot>,
}

impl Default for UiSurface {
    fn default() -> Self {
        Self {
            entity_to_taffy: Default::default(),
            taffy: Taffy::new(),
        }
    }
}

impl UiSurface {
    /// Retrieves the taffy node corresponding to given entity exists, or inserts a new taffy node into the layout if no corresponding node exists.
    /// Then convert the given `Style` and use it update the taffy node's style.
    pub fn upsert_node(&mut self, entity: Entity, style: &Style, context: &LayoutContext) {
        let mut added = false;
        let taffy = &mut self.taffy;
        let taffy_node = self.entity_to_taffy.entry(entity).or_insert_with(|| {
            added = true;
            taffy.new_leaf(convert::from_style(context, style)).unwrap()
        });

        if !added {
            self.taffy
                .set_style(*taffy_node, convert::from_style(context, style))
                .unwrap();
        }
    }

    /// Update the `MeasureFunc` of the taffy node corresponding to the given [`Entity`].
    pub fn update_measure(&mut self, entity: Entity, measure_func: taffy::node::MeasureFunc) {
        let taffy_node = self.entity_to_taffy.get(&entity).unwrap();
        self.taffy.set_measure(*taffy_node, Some(measure_func)).ok();
    }

    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(&mut self, entity: Entity, children: &Children) {
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
    pub fn try_remove_measure(&mut self, entity: Entity) {
        if let Some(taffy_node) = self.entity_to_taffy.get(&entity) {
            self.taffy.set_measure(*taffy_node, None).unwrap();
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
}

#[derive(Debug)]
pub enum LayoutError {
    InvalidHierarchy,
    TaffyError(taffy::error::TaffyError),
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
#[allow(clippy::too_many_arguments)]
pub fn ui_layout_system(
    mut ui_surface: ResMut<UiSurface>,
    mut camera_to_root: ResMut<UiCameraToRoot>,
    default_camera: ResMut<UiDefaultCamera>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut removed_ui_nodes: RemovedComponents<Node>,
    default_root_node_query: Query<Entity, (With<Node>, Without<Parent>, Without<UiTargetCamera>)>,
    root_uinode_query: Query<(Entity, &UiTargetCamera), (With<Node>, Without<Parent>)>,
    style_query: Query<(Entity, Ref<Style>), With<Node>>,
    mut measure_query: Query<(Entity, &mut ContentSize)>,
    children_query: Query<(Entity, Ref<Children>), With<Node>>,
    mut node_transform_query: Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
) {
    // clean up removed nodes
    ui_surface.remove_entities(removed_ui_nodes.iter());

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_content_sizes.iter() {
        ui_surface.try_remove_measure(entity);
    }

    // If an entities `Children` component is removed, remove the children of its associated Taffy node if one exists.
    for entity in removed_children.iter() {
        ui_surface.try_remove_children(entity);
    }

    // Need to modify UI stack to fix this
    let Some(&UiLayoutRoot { context, perform_full_update, .. }) = camera_to_root.values().next() else {
        // No layout roots, nothing to update so return.
        return;
    };

    if perform_full_update {
        // update all nodes
        for (entity, style) in style_query.iter() {
            ui_surface.upsert_node(entity, &style, &context);
        }

        for (entity, children) in &children_query {
            ui_surface.update_children(entity, &children);
        }
    } else {
        for (entity, style) in style_query.iter() {
            if style.is_changed() {
                ui_surface.upsert_node(entity, &style, &context);
            }

            for (entity, children) in &children_query {
                if children.is_changed() {
                    ui_surface.update_children(entity, &children);
                }
            }
        }
    }

    for (entity, mut content_size) in measure_query.iter_mut() {
        if let Some(measure_func) = content_size.measure_func.take() {
            ui_surface.update_measure(entity, measure_func);
        }
    }

    for layout_root in camera_to_root.values_mut() {
        layout_root.root_uinodes.clear();
        let _ = ui_surface.taffy.set_children(layout_root.taffy_root, &[]);
    }

    if let Some(default_root) = default_camera
        .entity
        .and_then(|default_camera| camera_to_root.get_mut(&default_camera))
    {
        default_root
            .root_uinodes
            .extend(default_root_node_query.iter());
        let taffy_children = default_root
            .root_uinodes
            .iter()
            .map(|e| *ui_surface.entity_to_taffy.get(e).unwrap())
            .collect::<Vec<TaffyNode>>();
        ui_surface
            .taffy
            .set_children(default_root.taffy_root, &taffy_children)
            .ok();
    }

    for (root_uinode, camera) in root_uinode_query.iter() {
        if let Some(layout_root) = camera_to_root.get_mut(&camera.entity) {
            layout_root.root_uinodes.push(root_uinode);
            let taffy_root = layout_root.taffy_root;
            let taffy_child = *ui_surface.entity_to_taffy.get(&root_uinode).unwrap();
            ui_surface.taffy.add_child(taffy_root, taffy_child).ok();
        }
    }

    // compute layouts
    for root in camera_to_root.values() {
        ui_surface
            .taffy
            .compute_layout(root.taffy_root, Size::MAX_CONTENT)
            .unwrap();
    }

    let to_logical = |v| (context.inverse_target_scale_factor * v as f64) as f32;

    // PERF: try doing this incrementally
    for (entity, mut node, mut transform, parent) in &mut node_transform_query {
        let layout = ui_surface.get_layout(entity).unwrap();
        let new_size = Vec2::new(
            to_logical(layout.size.width),
            to_logical(layout.size.height),
        );
        // only trigger change detection when the new value is different
        if node.calculated_size != new_size {
            node.calculated_size = new_size;
        }
        let mut new_position = transform.translation;
        new_position.x = to_logical(layout.location.x + layout.size.width / 2.0);
        new_position.y = to_logical(layout.location.y + layout.size.height / 2.0);
        if let Some(parent) = parent {
            if let Ok(parent_layout) = ui_surface.get_layout(**parent) {
                new_position.x -= to_logical(parent_layout.size.width / 2.0);
                new_position.y -= to_logical(parent_layout.size.height / 2.0);
            }
        }
        // only trigger change detection when the new value is different
        if transform.translation != new_position {
            transform.translation = new_position;
        }
    }
}
