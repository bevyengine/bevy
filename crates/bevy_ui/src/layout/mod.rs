mod convert;
pub mod debug;

use crate::{ContentSize, Node, Style, UiScale};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    event::EventReader,
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
use bevy_window::{PrimaryWindow, Window, WindowResolution, WindowScaleFactorChanged};
use std::fmt;
use taffy::{node::MeasureFunc, prelude::Size, style_helpers::TaffyMaxContent, Taffy};

type TaffyKey = taffy::node::Node;

/// Identifier for the UI node's associated entry in the UI's layout tree.
///
/// Users can only instantiate null nodes using `UiKey::default()`, the keys are set and managed internally by [`super::layout::ui_layout_system`].
/// All UI nodes must have this component.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct UiKey {
    // The id of the taffy node associated with the entity possessing this component.
    #[reflect(ignore)]
    taffy_key: TaffyKey,
}

impl UiKey {
    // Returns the id of the taffy node associated with the entity that has this component.
    pub fn get(&self) -> TaffyKey {
        self.taffy_key
    }

    // A null `UiKey` signifies that a UI node entity does not have an associated Taffy node in the UI layout tree.
    pub fn is_null(&self) -> bool {
        self.taffy_key == Default::default()
    }
}

pub struct LayoutContext {
    pub scale_factor: f64,
    pub physical_size: Vec2,
    pub min_size: f32,
    pub max_size: f32,
}

impl LayoutContext {
    /// create new a [`LayoutContext`] from the window's physical size and scale factor
    fn new(physical_size: Vec2, scale_factor: f64) -> Self {
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
    entity_to_taffy: HashMap<Entity, TaffyKey>,
    window_nodes: HashMap<Entity, TaffyKey>,
    taffy: Taffy,
}

fn _assert_send_sync_ui_surface_impl_safe() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<HashMap<Entity, TaffyKey>>();
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
        let mut taffy = Taffy::new();
        taffy.disable_rounding();
        Self {
            entity_to_taffy: Default::default(),
            window_nodes: Default::default(),
            taffy,
        }
    }
}

impl UiSurface {
    /// Returns a key identifying the internal layout tree node associated with the given `Entity`
    /// or `None` if no such association exists.
    #[inline]
    pub fn get_key(&self, entity: Entity) -> Option<TaffyKey> {
        self.entity_to_taffy.get(&entity).copied()
    }

    /// Inserts a node into the Taffy layout, associates it with the given entity, and returns its taffy id.
    pub fn insert_node(&mut self, uinode: Entity) -> TaffyKey {
        // It's possible for a user to overwrite an active `UiKey` component with `UiKey::default()`.
        // In which case we reuse the existing taffy node.
        *self
            .entity_to_taffy
            .entry(uinode)
            .or_insert_with(|| self.taffy.new_leaf(taffy::style::Style::default()).unwrap())
    }

    /// Converts the given Bevy UI `Style` to a taffy style and applies it to the taffy node with identifier `key`.
    pub fn set_style(&mut self, key: TaffyKey, style: &Style, layout_context: &LayoutContext) {
        self.taffy
            .set_style(key, convert::from_style(style, layout_context))
            .unwrap();
    }

    /// Sets the `MeasureFunc` for the taffy node `key` to `measure`.
    pub fn set_measure(&mut self, key: TaffyKey, measure: MeasureFunc) {
        self.taffy.set_measure(key, Some(measure)).unwrap();
    }

    /// Update the children of the taffy node `parent_key` .
    pub fn update_children(&mut self, parent_key: TaffyKey, children: &Children) {
        let mut taffy_children = Vec::with_capacity(children.len());
        for &child in children {
            if let Some(child_key) = self.get_key(child) {
                taffy_children.push(child_key);
            } else {
                warn!(
                    "Unstyled child in a UI entity hierarchy. You are using an entity \
without UI components as a child of an entity with UI components, results may be unexpected."
                );
            }
        }

        self.taffy
            .set_children(parent_key, &taffy_children)
            .unwrap();
    }

    /// Removes children from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_children(&mut self, parent: Entity) {
        if let Some(key) = self.get_key(parent) {
            self.taffy.set_children(key, &[]).unwrap();
        }
    }

    /// Removes the measure from the entity's taffy node if it exists. Does nothing otherwise.
    pub fn try_remove_measure(&mut self, entity: Entity) {
        if let Some(key) = self.get_key(entity) {
            self.taffy.set_measure(key, None).unwrap();
        }
    }

    /// Retrieve or insert the root layout node and update its size to match the size of the window.
    pub fn update_window(&mut self, window: Entity, window_resolution: &WindowResolution) {
        let taffy = &mut self.taffy;
        let key = self
            .window_nodes
            .entry(window)
            .or_insert_with(|| taffy.new_leaf(taffy::style::Style::default()).unwrap());

        taffy
            .set_style(
                *key,
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
        children: impl Iterator<Item = TaffyKey>,
    ) {
        let window_key = self.window_nodes.get(&parent_window).unwrap();
        let child_keys = children.collect::<Vec<TaffyKey>>();
        self.taffy.set_children(*window_key, &child_keys).unwrap();
    }

    /// Compute the layout for each window entity's corresponding root node in the layout.
    pub fn compute_window_layouts(&mut self) {
        for window_key in self.window_nodes.values() {
            self.taffy
                .compute_layout(*window_key, Size::MAX_CONTENT)
                .unwrap();
        }
    }

    /// Removes each entity from the internal map and then removes their associated node from taffy
    pub fn remove_entities(&mut self, entities: impl IntoIterator<Item = Entity>) {
        for entity in entities {
            if let Some(key) = self.entity_to_taffy.remove(&entity) {
                self.taffy.remove(key).unwrap();
            }
        }
    }
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
            Query<Entity, (With<UiKey>, Without<Parent>)>,
            Query<&Children, With<UiKey>>,
        ),
    )>,
// =======
//     root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
//     style_query: Query<(Entity, Ref<Style>), With<Node>>,
//     mut measure_query: Query<(Entity, &mut ContentSize)>,
//     children_query: Query<(Entity, Ref<Children>), With<Node>>,
//     just_children_query: Query<&Children>,
//     mut removed_children: RemovedComponents<Children>,
//     mut removed_content_sizes: RemovedComponents<ContentSize>,
//     mut node_transform_query: Query<(&mut Node, &mut Transform)>,
//     mut removed_nodes: RemovedComponents<Node>,
// >>>>>>> main
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
            return;
        };

    // Set the `window_changed` flag if the primary window was resized or created since the last UI update.
    let window_changed = window_resized_events
        .iter()
        .map(|resized| resized.window)
        .chain(window_created_events.iter().map(|created| created.window))
        .any(|window| window == primary_window_entity);

//<<<<<<< HEAD
    // clean up removed nodes first
    ui_surface.remove_entities(removed_nodes.iter());

    let mut ids_query = ui_queries_param_set.p0();
    for (entity, mut uikey) in ids_query.iter_mut() {
        // Users can only instantiate `UiKey` components containing a null key with `UiKey::default()`
        if uikey.is_null() {
            uikey.taffy_key = ui_surface.insert_node(entity);
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

    let (mut node_transform_query, children_query, mut measure_query, style_query, root_node_uikey_query, root_node_query, just_children_query) =
        ui_queries_param_set.p1();

    // update children
    for (uikey, children) in &children_query {
        if children.is_changed() {
            ui_surface.update_children(uikey.get(), &children);
        }
    }

// =======
//     // update window root nodes
//     for (entity, window) in windows.iter() {
//         ui_surface.update_window(entity, &window.resolution);
//     }

//     let scale_factor = logical_to_physical_factor * ui_scale.0;

//     let layout_context = LayoutContext::new(scale_factor, physical_size);

//     if !scale_factor_events.is_empty() || ui_scale.is_changed() || resized {
//         scale_factor_events.clear();
//         // update all nodes
//         for (entity, style) in style_query.iter() {
//             ui_surface.upsert_node(entity, &style, &layout_context);
//         }
//     } else {
//         for (entity, style) in style_query.iter() {
//             if style.is_changed() {
//                 ui_surface.upsert_node(entity, &style, &layout_context);
//             }
//         }
//     }

//     for (entity, mut content_size) in measure_query.iter_mut() {
//         if let Some(measure_func) = content_size.measure_func.take() {
//             ui_surface.update_measure(entity, measure_func);
//         }
//     }

//     // clean up removed nodes
// >>>>>>> main
    

    // update window root nodes
    for (entity, window) in windows.iter() {
        ui_surface.update_window(entity, &window.resolution);
    }

    let scale_factor = logical_to_physical_factor * ui_scale.0;

    let layout_context = LayoutContext::new(physical_size, scale_factor);

    if !scale_factor_events.is_empty() || ui_scale.is_changed() || window_changed {
        scale_factor_events.clear();
        // update all nodes
        for (uikey, style) in style_query.iter() {
            ui_surface.set_style(uikey.get(), &style, &layout_context);
        }
    } else {
        for (uikey, style) in style_query.iter() {
            if style.is_changed() {
                ui_surface.set_style(uikey.get(), &style, &layout_context);
            }
        }
    }

    // The `Taffy` tree takes ownership of any new `MeasureFunc`s
    for (uikey, mut content_size) in measure_query.iter_mut() {
        if let Some(measure_func) = content_size.measure_func.take() {
            ui_surface.set_measure(uikey.get(), measure_func);
        }
    }

    // update window children (for now assuming all Nodes live in the primary window)
    ui_surface.set_window_children(
        primary_window_entity,
        root_node_uikey_query.iter().map(|uikey| uikey.get()),
    );

    // compute layouts
    ui_surface.compute_window_layouts();

    let inverse_target_scale_factor = 1. / scale_factor;

    fn update_uinode_geometry_recursive(
        entity: Entity,
        ui_surface: &UiSurface,
        node_transform_query: &mut Query<(&UiKey, &mut Node, &mut Transform)>,
        children_query: &Query<&Children, With<UiKey>>,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
        mut absolute_location: Vec2,
    ) {
        if let Ok((uikey, mut node, mut transform)) = node_transform_query.get_mut(entity) {
            let layout = ui_surface.taffy.layout(uikey.get()).unwrap();
            let layout_size = Vec2::new(layout.size.width, layout.size.height);
            let layout_location = Vec2::new(layout.location.x, layout.location.y);
            absolute_location += layout_location;
            let rounded_location = round_layout_coords(layout_location);
            let rounded_size = round_layout_coords(absolute_location + layout_size)
                - round_layout_coords(absolute_location);
            let new_size = inverse_target_scale_factor * rounded_size;
            let new_position =
                inverse_target_scale_factor * rounded_location + 0.5 * (new_size - parent_size);

            // only trigger change detection when the new values are different
            if node.calculated_size != new_size {
                node.calculated_size = new_size;
            }
            if transform.translation.truncate() != new_position {
                transform.translation = new_position.extend(0.);
            }
            if let Ok(children) = children_query.get(entity) {
                for &child_uinode in children {
                    update_uinode_geometry_recursive(
                        child_uinode,
                        ui_surface,
                        node_transform_query,
                        children_query,
                        inverse_target_scale_factor,
                        new_size,
                        absolute_location,
                    );
                }
            }
        }
    }

    for entity in root_node_query.iter() {
        update_uinode_geometry_recursive(
            entity,
            &ui_surface,
            &mut node_transform_query,
            &just_children_query,
            inverse_target_scale_factor as f32,
            Vec2::ZERO,
            Vec2::ZERO,
        );
    }
}

#[inline]
/// Round `value` to the closest whole integer, with ties (values with a fractional part equal to 0.5) rounded towards positive infinity.
fn round_ties_up(value: f32) -> f32 {
    if 0. <= value || value.fract() != 0.5 {
        // The `round` function rounds ties away from zero. For positive numbers "away from zero" is towards positive infinity.
        // So for all positive values, and negative values with a fractional part not equal to 0.5, `round` returns the correct result.
        value.round()
    } else {
        // In the remaining cases, where `value` is negative and its fractional part is equal to 0.5, we use `ceil` to round it up towards positive infinity.
        value.ceil()
    }
}

#[inline]
/// Rust `f32` only has support for rounding ties away from zero.
/// When rounding the layout coordinates we need to round ties up, otherwise we can gain a pixel.
/// For example consider a node with left and right bounds of -50.5 and 49.5 (width: 49.5 - (-50.5) == 100).
/// After rounding left and right away from zero we get -51 and 50 (width: 50 - (-51) == 101), gaining a pixel.
fn round_layout_coords(value: Vec2) -> Vec2 {
    Vec2 {
        x: round_ties_up(value.x),
        y: round_ties_up(value.y),
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::world::World;
    use bevy_math::Vec2;

    use crate::layout::convert::from_style;
    use crate::UiSurface;

    #[test]
    fn uisurface_insert_and_remove_nodes() {
        let mut world = World::new();
        let mut ui_surface = UiSurface::default();
        let uinode_a = world.spawn_empty().id();
        let uinode_b = world.spawn_empty().id();
        let context = crate::LayoutContext::new(Vec2::new(800., 600.), 1.0);
        let style_a = crate::Style::default();
        let style_b = crate::Style {
            flex_direction: crate::FlexDirection::Column,
            ..Default::default()
        };
        let taffystyle_a = from_style(&style_a, &context);
        let taffystyle_b = from_style(&style_a, &context);
        let taffykey_a = ui_surface.insert_node(uinode_a);
        let taffykey_b = ui_surface.insert_node(uinode_b);
        ui_surface.set_style(taffykey_a, &style_a, &context);
        ui_surface.set_style(taffykey_b, &style_b, &context);
        assert_eq!(taffykey_a, ui_surface.get_key(uinode_a).unwrap());
        assert_eq!(taffykey_b, ui_surface.get_key(uinode_b).unwrap());

        // The should be be two nodes in the layout
        assert_eq!(ui_surface.entity_to_taffy.len(), 2);

        // The ids for the associated taffy nodes should be different
        assert_ne!(taffykey_a, taffykey_b);

        // Check the UI nodes each have an associated taffy node with the correct style
        assert_eq!(ui_surface.taffy.style(taffykey_a).unwrap(), &taffystyle_a);
        assert_eq!(ui_surface.taffy.style(taffykey_b).unwrap(), &taffystyle_b);

        // Swap the styles of the associated nodes
        ui_surface.set_style(taffykey_a, &style_b, &context);
        ui_surface.set_style(taffykey_b, &style_a, &context);

        // The styles should be swapped
        assert_eq!(ui_surface.taffy.style(taffykey_a).unwrap(), &taffystyle_b);
        assert_eq!(ui_surface.taffy.style(taffykey_b).unwrap(), &taffystyle_a);

        // But the ids of the associated nodes should be unchanged
        assert_eq!(ui_surface.get_key(uinode_a).unwrap(), taffykey_a);
        assert_eq!(ui_surface.get_key(uinode_b).unwrap(), taffykey_b);

        // There should still be exactly two nodes in the layout
        assert_eq!(ui_surface.entity_to_taffy.len(), 2);

        // Remove the uinodes from the layout and check styles are also removed
        ui_surface.remove_entities([uinode_a, uinode_b]);

        assert!(ui_surface.entity_to_taffy.is_empty());
        assert!(ui_surface.taffy.style(taffykey_a).is_err());
        assert!(ui_surface.taffy.style(taffykey_b).is_err());
    }
}
