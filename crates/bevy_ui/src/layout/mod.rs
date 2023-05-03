mod convert;

use crate::{ContentSize, NodeSize, Style, UiScale};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    event::EventReader,
    prelude::Component,
    query::{Added, Changed, With, Without},
    removal_detection::RemovedComponents,
    system::{Query, Res, ResMut, Resource},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window, WindowScaleFactorChanged};
use std::fmt;
use taffy::{prelude::Size, style_helpers::TaffyMaxContent, tree::LayoutTree, Taffy};

#[derive(Component, Default, Debug, Reflect)]
pub struct Node {
    #[reflect(ignore)]
    key: taffy::node::Node,
}

#[derive(Resource, Default)]
pub struct UiContext(pub Option<LayoutContext>);

pub struct LayoutContext {
    pub require_full_update: bool,
    pub scale_factor: f64,
    pub logical_size: Vec2,
    pub physical_size: Vec2,
    pub physical_to_logical_factor: f64,
    pub min_size: f32,
    pub max_size: f32,
}

impl LayoutContext {
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    fn new(scale_factor: f64, physical_size: Vec2, require_full_update: bool) -> Self {
        let physical_to_logical_factor = 1. / scale_factor;
        Self {
            require_full_update,
            scale_factor,
            logical_size: physical_size * physical_to_logical_factor as f32,
            physical_size,
            min_size: physical_size.x.min(physical_size.y),
            max_size: physical_size.x.max(physical_size.y),
            physical_to_logical_factor,
        }
    }
}

#[derive(Resource)]
pub struct UiSurface {
    entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    window_node: taffy::node::Node,
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
            .field("window_node", &self.window_node)
            .finish()
    }
}

impl Default for UiSurface {
    fn default() -> Self {
        Self {
            entity_to_taffy: Default::default(),
            window_node: Default::default(),
            taffy: Taffy::new(),
        }
    }
}

impl UiSurface {
    pub fn update_style(
        &mut self,
        taffy_node: taffy::node::Node,
        style: &Style,
        context: &LayoutContext,
    ) {
        self.taffy
            .set_style(taffy_node, convert::from_style(context, style))
            .ok();
    }

    /// Update the `MeasureFunc` of the taffy node corresponding to the given [`Entity`].
    pub fn update_measure(
        &mut self,
        taffy_node: taffy::node::Node,
        measure_func: taffy::node::MeasureFunc,
    ) {
        self.taffy.set_measure(taffy_node, Some(measure_func)).ok();
    }

    /// Update the children of the taffy node corresponding to the given [`Entity`].
    pub fn update_children(&mut self, parent: taffy::node::Node, children: &Children) {
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

        self.taffy.set_children(parent, &taffy_children).unwrap();
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

    /// Update window node so its size matches the resolution of the corresponding window
    pub fn update_window(&mut self, window_resolution: Vec2) {
        if self.window_node == taffy::node::Node::default() {
            self.window_node = self.taffy.new_leaf(taffy::style::Style::default()).unwrap();
        }
        self.taffy
            .set_style(
                self.window_node,
                taffy::style::Style {
                    size: taffy::geometry::Size {
                        width: taffy::style::Dimension::Points(window_resolution.x),
                        height: taffy::style::Dimension::Points(window_resolution.y),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    /// Set the ui node entities without a [`Parent`] as children to the root node in the taffy layout.
    pub fn set_window_children(&mut self, children: impl Iterator<Item = taffy::node::Node>) {
        let child_nodes = children.collect::<Vec<taffy::node::Node>>();
        self.taffy
            .set_children(self.window_node, &child_nodes)
            .unwrap();
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_window_layout(&mut self) {
        self.taffy
            .compute_layout(self.window_node, Size::MAX_CONTENT)
            .unwrap();
    }

    /// Removes each entity from the internal map and then removes their associated node from taffy
    pub fn remove_nodes(&mut self, entities: impl IntoIterator<Item = Entity>) {
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

/// Remove the corresponding taffy node for any entity that has its `Node` component removed.
pub fn clean_up_removed_ui_nodes(
    mut ui_surface: ResMut<UiSurface>,
    mut removed_nodes: RemovedComponents<Node>,
    mut removed_calculated_sizes: RemovedComponents<ContentSize>,
) {
    // clean up removed nodes
    ui_surface.remove_nodes(removed_nodes.iter());

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_calculated_sizes.iter() {
        ui_surface.try_remove_measure(entity);
    }
}

/// Insert a new taffy node into the layout for any entity that had a `Node` component added.
pub fn insert_new_ui_nodes(
    mut ui_surface: ResMut<UiSurface>,
    mut new_node_query: Query<(Entity, &mut Node), Added<Node>>,
) {
    for (entity, mut node) in new_node_query.iter_mut() {
        node.key = ui_surface
            .taffy
            .new_leaf(taffy::style::Style::DEFAULT)
            .unwrap();
        if let Some(old_key) = ui_surface.entity_to_taffy.insert(entity, node.key) {
            ui_surface.taffy.remove(old_key).ok();
        }
    }
}

/// Synchonise the Bevy and Taffy Parent-Children trees
pub fn synchonise_ui_children(
    mut flex_surface: ResMut<UiSurface>,
    mut removed_children: RemovedComponents<Children>,
    children_query: Query<(&Node, &Children), Changed<Children>>,
) {
    // Iterate through all entities with a removed `Children` component and if they have a corresponding Taffy node, remove their children from the Taffy tree.
    for entity in removed_children.iter() {
        flex_surface.try_remove_children(entity);
    }

    // Update the corresponding Taffy children of Bevy entities with changed `Children`
    for (node, children) in &children_query {
        flex_surface.update_children(node.key, children);
    }
}

pub fn update_ui_windows(
    mut resize_events: EventReader<bevy_window::WindowResized>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut ui_context: ResMut<UiContext>,
    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
) {
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
            ui_context.0 = None;
            return;
        };

    let require_full_update = ui_context.0.is_none()
        || resize_events
            .iter()
            .any(|resized_window| resized_window.window == primary_window_entity)
        || !scale_factor_events.is_empty()
        || ui_scale.is_changed();
    scale_factor_events.clear();

    let scale_factor = logical_to_physical_factor * ui_scale.scale;
    let context = LayoutContext::new(scale_factor, physical_size, require_full_update);
    ui_context.0 = Some(context);
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
#[allow(clippy::too_many_arguments)]
pub fn update_ui_layout(
    ui_context: ResMut<UiContext>,
    mut ui_surface: ResMut<UiSurface>,
    root_node_query: Query<&Node, (With<Style>, Without<Parent>)>,
    style_query: Query<(&Node, Ref<Style>)>,
    full_style_query: Query<(&Node, &Style)>,
    mut measure_query: Query<(&Node, &mut ContentSize)>,
) {
    let Some(ref layout_context) = ui_context.0 else {
        return
    };

    if layout_context.require_full_update {
        // update all nodes
        for (node, style) in full_style_query.iter() {
            ui_surface.update_style(node.key, style, layout_context);
        }
    } else {
        for (node, style) in style_query.iter() {
            if style.is_changed() {
                ui_surface.update_style(node.key, &style, layout_context);
            }
        }
    }

    for (node, mut content_size) in measure_query.iter_mut() {
        if let Some(measure_func) = content_size.measure_func.take() {
            ui_surface.update_measure(node.key, measure_func);
        }
    }

    // update window root nodes
    ui_surface.update_window(layout_context.physical_size);

    // update window children
    ui_surface.set_window_children(root_node_query.iter().map(|node| node.key));

    // compute layouts
    ui_surface.compute_window_layout();
}

pub fn update_ui_node_transforms(
    ui_surface: Res<UiSurface>,
    ui_context: ResMut<UiContext>,
    mut node_transform_query: Query<(&Node, &mut NodeSize, &mut Transform)>,
) {
    let Some(physical_to_logical_factor) = ui_context
        .0
        .as_ref()
        .map(|context|  context.physical_to_logical_factor)
    else {
        return;
    };

    let to_logical = |v| (physical_to_logical_factor * v as f64) as f32;

    // PERF: try doing this incrementally
    //for (node, mut node_size, mut transform) in &mut node_transform_query {
    node_transform_query
        .par_iter_mut()
        .for_each_mut(|(node, mut node_size, mut transform)| {
            let layout = ui_surface.taffy.layout(node.key).unwrap();
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

            let parent_key = ui_surface.taffy.parent(node.key).unwrap();
            if parent_key != ui_surface.window_node {
                let parent_layout = ui_surface.taffy.layout(parent_key).unwrap();
                new_position.x -= to_logical(parent_layout.size.width / 2.0);
                new_position.y -= to_logical(parent_layout.size.height / 2.0);
            }

            // only trigger change detection when the new value is different
            if transform.translation != new_position {
                transform.translation = new_position;
            }
        });
}

#[cfg(test)]
mod tests {
    use crate::clean_up_removed_ui_nodes;
    use crate::insert_new_ui_nodes;
    use crate::synchonise_ui_children;
    use crate::update_ui_layout;
    use crate::AlignItems;
    use crate::LayoutContext;
    use crate::Node;
    use crate::NodeSize;
    use crate::Style;
    use crate::UiContext;
    use crate::UiSurface;
    use bevy_ecs::prelude::*;
    use bevy_math::Vec2;
    use taffy::tree::LayoutTree;

    fn node_bundle() -> (Node, NodeSize, Style) {
        (Node::default(), NodeSize::default(), Style::default())
    }

    fn ui_schedule() -> Schedule {
        let mut ui_schedule = Schedule::default();
        ui_schedule.add_systems((
            clean_up_removed_ui_nodes.before(insert_new_ui_nodes),
            insert_new_ui_nodes.before(synchonise_ui_children),
            synchonise_ui_children.before(update_ui_layout),
            update_ui_layout,
        ));
        ui_schedule
    }

    #[test]
    fn test_insert_and_remove_node() {
        let mut world = World::new();
        world.init_resource::<UiSurface>();
        world.insert_resource(UiContext(Some(LayoutContext::new(
            3.0,
            Vec2::new(1000., 500.),
            true,
        ))));
        let mut ui_schedule = ui_schedule();

        // add ui node entity to world
        let entity = world.spawn(node_bundle()).id();

        // ui update
        ui_schedule.run(&mut world);

        let key = world.get::<Node>(entity).unwrap().key;
        let surface = world.resource::<UiSurface>();

        // ui node entity should be associated with a taffy node
        assert_eq!(surface.entity_to_taffy[&entity], key);

        // taffy node should be a child of the window node
        assert_eq!(surface.taffy.parent(key).unwrap(), surface.window_node);

        // despawn the ui node entity
        world.entity_mut(entity).despawn();

        ui_schedule.run(&mut world);

        let surface = world.resource::<UiSurface>();

        // the despawned entity's associated taffy node should also be removed
        assert!(!surface.entity_to_taffy.contains_key(&entity));

        // window node should have no children
        assert!(surface
            .taffy
            .children(surface.window_node)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_node_style_update() {
        let mut world = World::new();
        world.init_resource::<UiSurface>();
        world.insert_resource(UiContext(Some(LayoutContext::new(
            3.0,
            Vec2::new(1000., 500.),
            true,
        ))));
        let mut ui_schedule = ui_schedule();

        // add a ui node entity to the world and run the ui schedule to add a corresponding node to the taffy layout tree
        let entity = world.spawn(node_bundle()).id();
        ui_schedule.run(&mut world);

        // modify the ui node's style component and rerun the schedule
        world.get_mut::<Style>(entity).unwrap().align_items = AlignItems::Baseline;

        // don't want a full update
        world.insert_resource(UiContext(Some(LayoutContext::new(
            3.0,
            Vec2::new(1000., 500.),
            false,
        ))));

        ui_schedule.run(&mut world);

        // check the corresponding taffy node's style is also updated
        let ui_surface = world.resource::<UiSurface>();
        let key = ui_surface.entity_to_taffy[&entity];
        assert_eq!(
            ui_surface.taffy.style(key).unwrap().align_items,
            Some(taffy::style::AlignItems::Baseline)
        );
    }
}
