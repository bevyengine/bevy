mod convert;
pub mod debug;

use crate::{ContentSize, Node, Style, UiKey, UiScale};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    event::EventReader,
    query::{With, Without},
    removal_detection::RemovedComponents,
    system::{ParamSet, Query, Res, ResMut, Resource},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window, WindowResolution, WindowScaleFactorChanged};
use std::fmt;
use taffy::{prelude::Size, style_helpers::TaffyMaxContent, Taffy};

pub struct LayoutContext {
    pub scale_factor: f64,
    pub physical_size: Vec2,
    pub min_size: f32,
    pub max_size: f32,
}

impl LayoutContext {
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    fn new(scale_factor: f64, physical_size: Vec2) -> Self {
        Self {
            scale_factor,
            physical_size,
            min_size: physical_size.x.min(physical_size.y),
            max_size: physical_size.x.max(physical_size.y),
        }
    }
}

#[derive(Resource)]
pub struct UiSurface {
    entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    window_nodes: HashMap<Entity, taffy::node::Node>,
    taffy: Taffy,
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
            .field("window_nodes", &self.window_nodes)
            .finish()
    }
}

impl Default for UiSurface {
    fn default() -> Self {
        Self {
            entity_to_taffy: Default::default(),
            window_nodes: Default::default(),
            taffy: Taffy::new(),
        }
    }
}

impl UiSurface {
    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(&mut self, taffy_node: taffy::node::Node, children: &Children) {
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

    /// Retrieve or insert the root layout node and update its size to match the size of the window.
    pub fn update_window(&mut self, window: Entity, window_resolution: &WindowResolution) {
        let taffy = &mut self.taffy;
        let node = self
            .window_nodes
            .entry(window)
            .or_insert_with(|| taffy.new_leaf(taffy::style::Style::default()).unwrap());

        taffy
            .set_style(
                *node,
                taffy::style::Style {
                    size: taffy::geometry::Size {
                        width: taffy::style::Dimension::Points(
                            window_resolution.physical_width() as f32
                        ),
                        height: taffy::style::Dimension::Points(
                            window_resolution.physical_height() as f32,
                        ),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    /// Set the ui node entities without a [`Parent`] as children to the root node in the taffy layout.
    pub fn set_window_children(
        &mut self,
        parent_window: Entity,
        children: impl Iterator<Item = taffy::node::Node>,
    ) {
        let taffy_node = self.window_nodes.get(&parent_window).unwrap();
        let child_nodes = children.collect::<Vec<taffy::node::Node>>();
        self.taffy.set_children(*taffy_node, &child_nodes).unwrap();
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_window_layouts(&mut self) {
        for window_node in self.window_nodes.values() {
            self.taffy
                .compute_layout(*window_node, Size::MAX_CONTENT)
                .unwrap();
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
}

#[derive(Debug)]
pub enum LayoutError {
    InvalidHierarchy,
    TaffyError(taffy::error::TaffyError),
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
#[allow(clippy::too_many_arguments)]
pub fn ui_layout_system(
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    ui_scale: Res<UiScale>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
    mut window_resized_events: EventReader<bevy_window::WindowResized>,
    mut window_created_events: EventReader<bevy_window::WindowCreated>,
    mut ui_surface: ResMut<UiSurface>,
    mut removed_nodes: RemovedComponents<UiKey>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut ui_queries_param_set: ParamSet<(
        Query<(Entity, &mut UiKey)>,
        (
            Query<(&UiKey, &mut Node, &mut Transform)>,
            Query<(&UiKey, Ref<Children>)>,
            Query<(&UiKey, &mut ContentSize)>,
            Query<(&UiKey, Ref<Style>)>,
            Query<&UiKey, Without<Parent>>,
        ),
    )>,
) {
    // clean up removed nodes first
    ui_surface.remove_entities(removed_nodes.iter());

    let mut ui_keys_query = ui_queries_param_set.p0();
    for (entity, mut ui_key) in ui_keys_query.iter_mut() {
        // Users can only instantiate `UiKey` components containing a null key
        if ui_key.is_null() {
            ui_key.taffy_node = ui_surface
                .taffy
                .new_leaf(taffy::style::Style::default())
                .unwrap();
            if let Some(old_taffy_node) =
                ui_surface.entity_to_taffy.insert(entity, ui_key.taffy_node)
            {
                ui_surface.taffy.remove(old_taffy_node).unwrap();
            }
        }
    }

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_content_sizes.iter() {
        ui_surface.try_remove_measure(entity);
    }

    // remove children
    for entity in removed_children.iter() {
        ui_surface.try_remove_children(entity);
    }

    let (mut node_transform_query, children_query, mut measure_query, style_query, root_node_query) =
        ui_queries_param_set.p1();

    // update children
    for (ui_key, children) in &children_query {
        if children.is_changed() {
            ui_surface.update_children(ui_key.taffy_node, &children);
        }
    }

    // assume one window for time being...
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let (primary_window_entity, logical_to_physical_factor, physical_size) =
        if let Ok((entity, primary_window)) = primary_window.get_single() {
            (
                entity,
                primary_window.resolution.scale_factor(),
                Vec2::new(
                    primary_window.resolution.physical_width() as f32,
                    primary_window.resolution.physical_height() as f32,
                ),
            )
        } else {
            return;
        };

    let window_changed = window_resized_events
        .iter()
        .map(|resized| resized.window)
        .chain(window_created_events.iter().map(|created| created.window))
        .any(|window| window == primary_window_entity);

    // update window root nodes
    for (entity, window) in windows.iter() {
        ui_surface.update_window(entity, &window.resolution);
    }

    let scale_factor = logical_to_physical_factor * ui_scale.scale;

    let layout_context = LayoutContext::new(scale_factor, physical_size);

    if !scale_factor_events.is_empty() || ui_scale.is_changed() || window_changed {
        scale_factor_events.clear();
        // update all nodes
        for (ui_key, style) in style_query.iter() {
            ui_surface
                .taffy
                .set_style(
                    ui_key.taffy_node,
                    convert::from_style(&layout_context, &style),
                )
                .unwrap();
        }
    } else {
        for (ui_key, style) in style_query.iter() {
            if style.is_changed() {
                ui_surface
                    .taffy
                    .set_style(
                        ui_key.taffy_node,
                        convert::from_style(&layout_context, &style),
                    )
                    .unwrap();
            }
        }
    }

    for (ui_key, mut content_size) in measure_query.iter_mut() {
        if let Some(measure_func) = content_size.measure_func.take() {
            ui_surface
                .taffy
                .set_measure(ui_key.taffy_node, Some(measure_func))
                .ok();
        }
    }

    // update window children (for now assuming all Nodes live in the primary window)
    ui_surface.set_window_children(
        primary_window_entity,
        root_node_query.iter().map(|node| node.taffy_node),
    );

    // compute layouts
    ui_surface.compute_window_layouts();

    let physical_to_logical_factor = 1. / logical_to_physical_factor;

    let to_logical = |v| (physical_to_logical_factor * v as f64) as f32;

    let taffy_window = ui_surface.window_nodes.get(&primary_window_entity).unwrap();

    // PERF: try doing this incrementally
    for (ui_key, mut node_size, mut transform) in &mut node_transform_query {
        let layout = ui_surface.taffy.layout(ui_key.taffy_node).unwrap();
        let new_size = Vec2::new(
            to_logical(layout.size.width),
            to_logical(layout.size.height),
        );
        // only trigger change detection when the new value is different
        if node_size.calculated_size != new_size {
            node_size.calculated_size = new_size;
        }
        let mut new_position = transform.translation;
        new_position.x = to_logical(layout.location.x + layout.size.width / 2.0);
        new_position.y = to_logical(layout.location.y + layout.size.height / 2.0);

        let taffy_parent =
            taffy::tree::LayoutTree::parent(&ui_surface.taffy, ui_key.taffy_node).unwrap();
        if taffy_parent != *taffy_window {
            if let Ok(parent_layout) = ui_surface.taffy.layout(taffy_parent) {
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
