mod convert;
pub mod debug;

use crate::{ContentSize, Node, Style, UiScale};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::Component,
    query::{With, Without},
    removal_detection::RemovedComponents,
    system::{Commands, Query, Res, ResMut, Resource},
    world::Ref,
};
use bevy_hierarchy::{Children, Parent};
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
use bevy_window::{PrimaryWindow, Window};
use std::fmt;
use taffy::{prelude::Size, style_helpers::TaffyMaxContent, Taffy};

#[derive(Component, Debug, Copy, Clone)]
pub struct LayoutContext {
    pub physical_size: Vec2,
    pub scale_factor: f64,
    pub physical_to_logical_factor: f64,
}

impl Default for LayoutContext {
    fn default() -> Self {
        Self {
            physical_size: Vec2::new(800., 600.),
            scale_factor: 1.0,
            physical_to_logical_factor: 1.0,
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

    /// Retrieve or insert the root layout node and update its size to match the size of the window.
    pub fn update_window(&mut self, window: Entity, physical_size: Vec2) {
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
                        width: taffy::style::Dimension::Points(physical_size.x),
                        height: taffy::style::Dimension::Points(physical_size.y),
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
        children: impl Iterator<Item = Entity>,
    ) {
        let taffy_node = self.window_nodes.get(&parent_window).unwrap();
        let child_nodes = children
            .map(|e| *self.entity_to_taffy.get(&e).unwrap())
            .collect::<Vec<taffy::node::Node>>();
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

/// Track's window and scale factor changes and sets the [`LayoutContext`].
pub fn ui_windows_system(
    mut commands: Commands,
    ui_scale: Res<UiScale>,
    mut primary_window: Query<(Entity, &Window, Option<&mut LayoutContext>), With<PrimaryWindow>>,
) {
    let (primary_window_entity, logical_to_physical_factor, physical_size, maybe_layout_context) =
        if let Ok((entity, primary_window, maybe_layout_context)) = primary_window.get_single_mut()
        {
            (
                entity,
                primary_window.resolution.scale_factor(),
                Vec2::new(
                    primary_window.resolution.physical_width() as f32,
                    primary_window.resolution.physical_height() as f32,
                ),
                maybe_layout_context,
            )
        } else {
            return;
        };

    let scale_factor = logical_to_physical_factor * ui_scale.scale;
    let physical_to_logical_factor = logical_to_physical_factor.recip();
    let new_layout_context = LayoutContext {
        physical_size,
        scale_factor,
        physical_to_logical_factor,
    };

    if let Some(mut layout_context) = maybe_layout_context {
        if approx::relative_ne!(
            layout_context.physical_size.x,
            new_layout_context.physical_size.x
        ) && approx::relative_ne!(
            layout_context.physical_size.y,
            new_layout_context.physical_size.y
        ) && approx::relative_ne!(layout_context.scale_factor, new_layout_context.scale_factor)
        {
            *layout_context = new_layout_context;
        }
    } else {
        commands
            .entity(primary_window_entity)
            .insert(new_layout_context);
    }
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
#[allow(clippy::too_many_arguments)]
pub fn ui_layout_system(
    layout_context_query: Query<(Entity, Ref<LayoutContext>)>,
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
    let Ok((primary_window_entity, layout_context)) = layout_context_query.get_single() else {
        return;
    };

    // update window root nodes
    ui_surface.update_window(primary_window_entity, layout_context.physical_size);

    if layout_context.is_changed() {
        // update all nodes
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

    for (entity, mut content_size) in measure_query.iter_mut() {
        if let Some(measure_func) = content_size.measure_func.take() {
            ui_surface.update_measure(entity, measure_func);
        }
    }

    // clean up removed nodes
    ui_surface.remove_entities(removed_nodes.iter());

    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_content_sizes.iter() {
        ui_surface.try_remove_measure(entity);
    }

    // update window children (for now assuming all Nodes live in the primary window)
    ui_surface.set_window_children(primary_window_entity, root_node_query.iter());

    // update and remove children
    for entity in removed_children.iter() {
        ui_surface.try_remove_children(entity);
    }
    for (entity, children) in &children_query {
        if children.is_changed() {
            ui_surface.update_children(entity, &children);
        }
    }

    // compute layouts
    ui_surface.compute_window_layouts();

    let to_logical = |v| (layout_context.physical_to_logical_factor * v as f64) as f32;

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
                physical_size: Vec2::new(800., 600.),
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
            .window_nodes
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
