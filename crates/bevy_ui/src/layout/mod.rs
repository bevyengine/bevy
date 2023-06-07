mod convert;
pub mod debug;

use crate::{ContentSize, Node, Style};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::Component,
    query::{Added, With, Without},
    removal_detection::RemovedComponents,
    system::{ParamSet, Query, ResMut, Resource},
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
pub struct UiContext {
    /// Physical size of the target in pixels.
    pub physical_size: Vec2,
    /// Product of the target's scale factor and the camera's `UiScale`.
    pub combined_scale_factor: f64,
    /// Inverse of the target's scale factor.
    pub inverse_target_scale_factor: f64,
    /// The local `UiScale` for this layout
    pub ui_scale: f64,
}

impl UiContext {
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    pub(crate) fn new(physical_size: Vec2, target_scale_factor: f64, ui_scale: f64) -> Self {
        let combined_scale_factor = ui_scale * target_scale_factor;
        let inverse_target_scale_factor = target_scale_factor.recip();
        Self {
            physical_size,
            combined_scale_factor,
            inverse_target_scale_factor,
            ui_scale,
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

/// Describes a UI layout
#[derive(Debug)]
pub struct UiLayout {
    /// Root node of this layout's Taffy tree.
    pub(crate) taffy_root: TaffyNode,
    /// The physical size and scaling for this layout.
    pub(crate) context: UiContext,
    /// Update every node in the internal Taffy tree for this layout from its corresponding UI node's `Style` and `Children` components.
    pub(crate) needs_full_update: bool,
    /// Root uinodes are UI node entities without a `Parent` component.
    /// The Taffy nodes corresponding to each of the root uinodes are children of the taffy root for this layout
    pub(crate) root_uinodes: Vec<Entity>,
    /// Indicates the scale factor for this UI layout has been changed since the last update
    pub(crate) scale_factor_changed: bool,
}

impl UiLayout {
    pub(crate) fn new(taffy_root: TaffyNode, layout_context: UiContext) -> Self {
        Self {
            taffy_root,
            context: layout_context,
            needs_full_update: true,
            root_uinodes: vec![],
            scale_factor_changed: true,
        }
    }
}

/// The camera view that will show this UI layout
#[derive(Component, Debug, Reflect, FromReflect)]
pub struct UiView {
    pub entity: Entity,
}

#[derive(Resource)]
pub struct UiSurface {
    pub(crate) entity_to_taffy: HashMap<Entity, TaffyNode>,
    pub(crate) taffy: Taffy,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<HashMap<Entity, TaffyNode>>();
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
pub struct UiDefaultView {
    // Orphaned UI nodes without a `UiTargetCamera` component are added to the default camera's associated layout.
    pub entity: Option<Entity>,
}

///
#[derive(Resource, Default, Deref, DerefMut)]
pub struct UiLayouts {
    camera_to_root: HashMap<Entity, UiLayout>,
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
    /// Update the `MeasureFunc` of the taffy node corresponding to the given [`Entity`].
    pub fn update_measure(&mut self, entity: Entity, measure_func: taffy::node::MeasureFunc) {
        let taffy_node = self.entity_to_taffy.get(&entity).unwrap();
        self.taffy.set_measure(*taffy_node, Some(measure_func)).ok();
    }

    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(&mut self, taffy_node: TaffyNode, children: &Children) {
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

        self.taffy
            .set_children(taffy_node, &taffy_children)
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
    mut view_to_layout: ResMut<UiLayouts>,
    default_view: ResMut<UiDefaultView>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut removed_ui_nodes: RemovedComponents<Node>,
    default_root_node_query: Query<Entity, (With<Node>, Without<Parent>, Without<UiView>)>,
    root_uinode_query: Query<(Entity, &UiView), (With<Node>, Without<Parent>)>,
    mut measure_query: Query<(Entity, &mut ContentSize)>,
    uinode_query: Query<(Ref<Style>, Option<Ref<Children>>), With<Node>>,
    mut uinode_queries_paramset: ParamSet<(
        Query<Entity, Added<Node>>,
        Query<(&mut Node, &mut Transform)>,
    )>,
    children_query: Query<&Children>,
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

    let new_uinodes_query = uinode_queries_paramset.p0();
    // Insert new uinodes
    for uinode in new_uinodes_query.iter() {
        let taffy_node = ui_surface
            .taffy
            .new_leaf(taffy::style::Style::default())
            .unwrap();
        ui_surface.entity_to_taffy.insert(uinode, taffy_node);
    }

    // Clear root uinodes lists
    for layout in view_to_layout.values_mut() {
        layout.root_uinodes.clear();
        let _ = ui_surface.taffy.set_children(layout.taffy_root, &[]);
    }

    // Add uinodes without a `Parent` or a `UiView` to the default layout and make their associated Taffy node a child of the layout's root node.
    if let Some(default_layout) = default_view
        .entity
        .and_then(|default_camera| view_to_layout.get_mut(&default_camera))
    {
        default_layout
            .root_uinodes
            .extend(default_root_node_query.iter());
        let taffy_children = default_layout
            .root_uinodes
            .iter()
            .map(|e| *ui_surface.entity_to_taffy.get(e).unwrap())
            .collect::<Vec<TaffyNode>>();
        ui_surface
            .taffy
            .set_children(default_layout.taffy_root, &taffy_children)
            .ok();
    }

    // Add uinodes without a `Parent` to the layout corresponding to their `UiView` and make their associated Taffy node a child of the layout's root node.
    for (root_uinode, camera) in root_uinode_query.iter() {
        if let Some(layout_root) = view_to_layout.get_mut(&camera.entity) {
            layout_root.root_uinodes.push(root_uinode);
            let taffy_root = layout_root.taffy_root;
            let taffy_child = *ui_surface.entity_to_taffy.get(&root_uinode).unwrap();
            ui_surface.taffy.add_child(taffy_root, taffy_child).ok();
        }
    }

    fn taffy_tree_full_update_recursive(
        uinode: Entity,
        ui_surface: &mut UiSurface,
        context: &UiContext,
        uinode_query: &Query<(Ref<Style>, Option<Ref<Children>>), With<Node>>,
    ) {
        if let Ok((style, maybe_children)) = uinode_query.get(uinode) {
            let taffy_node = *ui_surface.entity_to_taffy.get(&uinode).unwrap();
            let _ = ui_surface
                .taffy
                .set_style(taffy_node, convert::from_style(context, &style));

            if let Some(children) = maybe_children {
                ui_surface.update_children(taffy_node, &children);

                for &child in &children {
                    taffy_tree_full_update_recursive(child, ui_surface, context, uinode_query);
                }
            }
        }
    }

    fn taffy_tree_update_changed_recursive(
        uinode: Entity,
        ui_surface: &mut UiSurface,
        context: &UiContext,
        uinode_query: &Query<(Ref<Style>, Option<Ref<Children>>), With<Node>>,
    ) {
        if let Ok((style, maybe_children)) = uinode_query.get(uinode) {
            if style.is_changed() {
                let taffy_node = *ui_surface.entity_to_taffy.get(&uinode).unwrap();
                let _ = ui_surface
                    .taffy
                    .set_style(taffy_node, convert::from_style(context, &style));
            }

            if let Some(children) = maybe_children {
                if children.is_changed() {
                    let taffy_node = *ui_surface.entity_to_taffy.get(&uinode).unwrap();
                    ui_surface.update_children(taffy_node, &children);
                }

                for &child in &children {
                    taffy_tree_update_changed_recursive(child, ui_surface, context, uinode_query);
                }
            }
        }
    }

    // Synchronise the Bevy and Taffy node's styles and parent-child hierarchy.
    for layout in view_to_layout.values() {
        if layout.needs_full_update {
            for &uinode in &layout.root_uinodes {
                taffy_tree_full_update_recursive(
                    uinode,
                    &mut ui_surface,
                    &layout.context,
                    &uinode_query,
                );
            }
        } else {
            for &uinode in &layout.root_uinodes {
                taffy_tree_update_changed_recursive(
                    uinode,
                    &mut ui_surface,
                    &layout.context,
                    &uinode_query,
                );
            }
        }
    }

    for (entity, mut content_size) in measure_query.iter_mut() {
        if let Some(measure_func) = content_size.measure_func.take() {
            ui_surface.update_measure(entity, measure_func);
        }
    }

    // compute layouts
    for layout in view_to_layout.values() {
        ui_surface
            .taffy
            .compute_layout(layout.taffy_root, Size::MAX_CONTENT)
            .unwrap();
    }

    fn update_uinode_geometry_recursive(
        uinode: Entity,
        ui_surface: &UiSurface,
        uinode_geometry_query: &mut Query<(&mut Node, &mut Transform)>,
        children_query: &Query<&Children>,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
    ) {
        if let Ok((mut node, mut transform)) = uinode_geometry_query.get_mut(uinode) {
            let layout = ui_surface.get_layout(uinode).unwrap();
            let size =
                Vec2::new(layout.size.width, layout.size.height) * inverse_target_scale_factor;
            let position = Vec2::new(layout.location.x, layout.location.y)
                * inverse_target_scale_factor
                + 0.5 * (size - parent_size);

            // only trigger change detection when the new values are different
            if node.calculated_size != size {
                node.calculated_size = size;
            }
            if transform.translation.truncate() != position {
                transform.translation = position.extend(0.);
            }

            if let Ok(children) = children_query.get(uinode) {
                for &child_uinode in children {
                    update_uinode_geometry_recursive(
                        child_uinode,
                        ui_surface,
                        uinode_geometry_query,
                        children_query,
                        inverse_target_scale_factor,
                        size,
                    );
                }
            }
        }
    }

    let mut uinode_geometries_query = uinode_queries_paramset.p1();
    for layout in view_to_layout.values() {
        for &root_uinode in &layout.root_uinodes {
            update_uinode_geometry_recursive(
                root_uinode,
                &ui_surface,
                &mut uinode_geometries_query,
                &children_query,
                layout.context.inverse_target_scale_factor as f32,
                Vec2::ZERO,
            );
        }
    }
}
