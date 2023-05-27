mod convert;
pub mod debug;

use crate::{ContentSize, Node, Style, UiScale};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::Component,
    query::{With, Without},
    reflect::ReflectComponent,
    removal_detection::RemovedComponents,
    system::{ParamSet, Query, Res, ResMut, Resource},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use bevy_window::Window;
use std::fmt;
use taffy::{prelude::Size, style_helpers::TaffyMaxContent, Taffy};

/// Marks an entity as `UI root entity` with an associated root Taffy node and holds the resolution and scale factor information necessary to compute a UI layout.
#[derive(Component, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct LayoutContext {
    /// The size of the root node in the layout tree.
    ///
    /// Should match the size of the output window in physical pixels of the display device.
    pub root_node_size: Vec2,
    /// [`Style`] properties of UI node entites with `Val::Px` values are multiplied by the `combined_scale_factor` before they are copied to the Taffy layout tree.
    ///
    /// `combined_scale_factor` is calculated by multiplying together the `scale_factor` of the output window and [`crate::UiScale`].
    pub combined_scale_factor: f64,
    /// After a UI layout has been computed, the layout coordinates are multiplied by `layout_to_logical_factor` to determine the final size of each UI Node entity to be stored in its [`Node`] component.
    ///
    /// `layout_to_logical_factor` is the reciprocal of the target window's `scale_factor` and doesn't include [`crate::UiScale`].
    pub layout_to_logical_factor: f64,
}

impl Default for LayoutContext {
    fn default() -> Self {
        Self {
            root_node_size: Vec2::new(800., 600.),
            combined_scale_factor: 1.0,
            layout_to_logical_factor: 1.0,
        }
    }
}

impl LayoutContext {
    fn relative_ne(&self, other: &Self) -> bool {
        approx::relative_ne!(self.root_node_size.x, other.root_node_size.x,)
            || approx::relative_ne!(self.root_node_size.y, other.root_node_size.y,)
            || approx::relative_ne!(self.combined_scale_factor, other.combined_scale_factor)
            || approx::relative_ne!(
                self.layout_to_logical_factor,
                other.layout_to_logical_factor,
            )
    }
}

pub struct UiLayout {}

#[derive(Resource)]
pub struct UiSurface {
    entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    root_nodes: HashMap<Entity, taffy::node::Node>,
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
            .field("window_nodes", &self.root_nodes)
            .finish()
    }
}

impl Default for UiSurface {
    fn default() -> Self {
        Self {
            entity_to_taffy: Default::default(),
            root_nodes: Default::default(),
            taffy: Taffy::new(),
        }
    }
}

impl UiSurface {
    /// Retrieves the taffy node corresponding to given entity exists, or inserts a new taffy node into the layout if no corresponding node exists.
    /// Then convert the given [`Style`] and use it update the taffy node's style.
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

    /// Retrieve or insert the root layout node and update its size to match the size of the window.
    pub fn update_root_nodes(&mut self, window: Entity, physical_size: Vec2) {
        let taffy = &mut self.taffy;
        let node = self
            .root_nodes
            .entry(window)
            .or_insert_with(|| taffy.new_leaf(taffy::style::Style::default()).unwrap());

        taffy
            .set_style(
                *node,
                taffy::style::Style {
                    size: taffy::geometry::Size {
                        width: taffy::style::Dimension::Points(physical_size.x),
                        height: taffy::style::Dimension::Points(physical_size.y),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    /// Set the ui node entities without a [`Parent`] as children to the root node in the taffy layout.
    pub fn set_root_nodes_children(
        &mut self,
        parent_window: Entity,
        children: impl Iterator<Item = Entity>,
    ) {
        let taffy_node = self.root_nodes.get(&parent_window).unwrap();
        let child_nodes = children
            .map(|e| *self.entity_to_taffy.get(&e).unwrap())
            .collect::<Vec<taffy::node::Node>>();
        self.taffy.set_children(*taffy_node, &child_nodes).unwrap();
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_window_layouts(&mut self) {
        for window_node in self.root_nodes.values() {
            self.taffy
                .compute_layout(*window_node, Size::MAX_CONTENT)
                .unwrap();
        }
    }

    /// Removes each entity from the internal map and then removes their associated node from Taffy
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
    ui_scale: Res<UiScale>,
    mut removed_layouts: RemovedComponents<LayoutContext>,
    mut context_params: ParamSet<(
        Query<(&Window, &mut LayoutContext)>,
        Query<(Entity, Ref<LayoutContext>)>,
    )>,
    mut ui_surface: ResMut<UiSurface>,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    style_query: Query<(Entity, Ref<Style>), With<Node>>,
    mut measure_query: Query<(Entity, &mut ContentSize)>,
    children_query: Query<(Entity, Ref<Children>), With<Node>>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut node_transform_query: Query<(Entity, &mut Node, &mut Transform, Option<&Parent>)>,
    mut removed_nodes: RemovedComponents<Node>,
) {
    // If a UI root entity is deleted, its associated Taffy root node must also be deleted.
    for entity in removed_layouts.iter() {
        if let Some(taffy_node) = ui_surface.root_nodes.remove(&entity) {
            let _ = ui_surface.taffy.remove(taffy_node);
        }
    }

    let mut windows_query = context_params.p0();
    for (window, mut layout_context) in &mut windows_query {
        let new_layout_context = LayoutContext {
            root_node_size: Vec2::new(
                window.resolution.physical_width() as f32,
                window.resolution.physical_height() as f32,
            ),
            combined_scale_factor: window.resolution.scale_factor() * ui_scale.scale,
            layout_to_logical_factor: window.resolution.scale_factor().recip(),
        };
        if layout_context.relative_ne(&new_layout_context) {
            *layout_context = new_layout_context;
        }
    }

    let ui_roots_query = context_params.p1();

    // If more than one UI root exists, only the first from the query is updated.
    let Some((ui_root_entity, layout_context)) = ui_roots_query.iter().next() else {
        return;
    };

    ui_surface.update_root_nodes(ui_root_entity, layout_context.root_node_size);

    if layout_context.is_changed() {
        // Update all nodes
        //
        // All nodes have to be updated on changes to the `LayoutContext` so any viewport values can be recalculated.
        for (entity, style) in style_query.iter() {
            ui_surface.upsert_node(entity, &style, &layout_context);
        }
    } else {
        for (entity, style) in style_query.iter() {
            if style.is_changed() {
                ui_surface.upsert_node(entity, &style, &layout_context);
            }
        }
    }

    // Add new `MeasureFunc`s to the `Taffy` layout tree
    for (entity, mut content_size) in measure_query.iter_mut() {
        if let Some(measure_func) = content_size.measure_func.take() {
            // The `ContentSize` component only holds a `MeasureFunc` temporarily until it reaches here and is moved into the `Taffy` layout tree.
            ui_surface.update_measure(entity, measure_func);
        }
    }

    // Only entities with a `Node` component are considered UI node entities.
    // When a `Node` component of an entity is removed, the Taffy node associated with that entity must be deleted from the Taffy layout tree.
    ui_surface.remove_entities(removed_nodes.iter());

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding Taffy node.
    for entity in removed_content_sizes.iter() {
        ui_surface.try_remove_measure(entity);
    }

    // Set the associated Taffy nodes of UI node entities without a `Parent` component to be children of the UI's root Taffy node
    ui_surface.set_root_nodes_children(ui_root_entity, root_node_query.iter());

    // Remove the associated Taffy children of entities which had their `Children` component removed since the last layout update
    //
    // This must be performed before `update_children` to account for cases where a `Children` component has been both removed and then reinserted between layout updates.
    for entity in removed_children.iter() {
        ui_surface.try_remove_children(entity);
    }

    // If the `Children` of a UI node entity have been changed since the last layout update, the children of the associated Taffy node must be updated.
    for (entity, children) in &children_query {
        if children.is_changed() {
            ui_surface.update_children(entity, &children);
        }
    }

    // compute layouts
    ui_surface.compute_window_layouts();

    // `layout_to_logical_factor` is the reciprocal of the `scale_factor` of the target window, and does not include `UiScale`.
    let to_logical = |v| (layout_context.layout_to_logical_factor * v as f64) as f32;

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

#[cfg(test)]
mod tests {
    use crate::debug;
    use crate::prelude::*;
    use crate::ui_layout_system;
    use crate::LayoutContext;
    use crate::UiSurface;
    use bevy_ecs::schedule::Schedule;
    use bevy_ecs::world::World;
    use bevy_math::Vec2;
    use bevy_utils::prelude::default;
    use taffy::tree::LayoutTree;

    #[test]
    fn spawn_and_despawn_ui_node() {
        let mut world = World::new();
        world.init_resource::<UiSurface>();

        let layout_entity = world
            .spawn(LayoutContext {
                root_node_size: Vec2::new(800., 600.),
                ..default()
            })
            .id();

        let ui_node = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(25.),
                    ..default()
                },
                ..default()
            })
            .id();

        let mut ui_schedule = Schedule::default();
        ui_schedule.add_systems(ui_layout_system);
        ui_schedule.run(&mut world);

        let ui_surface = world.resource::<UiSurface>();

        // `layout_entity` should have an associated Taffy root node
        let taffy_root = *ui_surface
            .root_nodes
            .get(&layout_entity)
            .expect("Window node not found.");

        // `ui_node` should have an associated Taffy node
        let taffy_node = *ui_surface
            .entity_to_taffy
            .get(&ui_node)
            .expect("UI node entity should have an associated Taffy node after layout update");

        // `window_node` should be the only child of `taffy_root`
        assert_eq!(ui_surface.taffy.child_count(taffy_root).unwrap(), 1);
        assert!(
            ui_surface
                .taffy
                .children(taffy_root)
                .unwrap()
                .contains(&taffy_node),
            "Root UI Node entity's corresponding Taffy node is not a child of the root Taffy node."
        );

        // `taffy_root` should be the parent of `window_node`
        assert_eq!(
            ui_surface.taffy.parent(taffy_node),
            Some(taffy_root),
            "Root UI Node entity's corresponding Taffy node is not a child of the root Taffy node."
        );

        ui_schedule.run(&mut world);

        let derived_size = world.get::<Node>(ui_node).unwrap().calculated_size;
        approx::assert_relative_eq!(derived_size.x, 200.);
        approx::assert_relative_eq!(derived_size.y, 600.);

        world.despawn(ui_node);
        ui_schedule.run(&mut world);
        let ui_surface = world.resource::<UiSurface>();

        // `ui_node`'s associated taffy node should be deleted
        assert!(
            !ui_surface.entity_to_taffy.contains_key(&ui_node),
            "Despawned UI node has an associated Taffy node after layout update"
        );

        // `taffy_root` should have no remaining children
        assert_eq!(
            ui_surface.taffy.child_count(taffy_root).unwrap(),
            0,
            "Taffy root node has children after despawning all root UI nodes."
        );

        debug::print_ui_layout_tree(ui_surface);
    }
}
