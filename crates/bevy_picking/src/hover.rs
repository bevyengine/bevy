//! Determines which entities are being hovered by which pointers.
//!
//! The most important type in this module is the [`HoverMap`], which maps pointers to the entities
//! they are hovering over.

use alloc::collections::BTreeMap;
use core::fmt::Debug;
use std::collections::HashSet;

use crate::{
    backend::{self, HitData},
    pointer::{PointerAction, PointerId, PointerInput, PointerInteraction, PointerPress},
    Pickable,
};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{entity::EntityHashSet, prelude::*};
use bevy_math::FloatOrd;
use bevy_platform::collections::HashMap;
use bevy_reflect::prelude::*;

type DepthSortedHits = Vec<(Entity, HitData)>;

/// Events returned from backends can be grouped with an order field. This allows picking to work
/// with multiple layers of rendered output to the same render target.
type PickLayer = FloatOrd;

/// Maps [`PickLayer`]s to the map of entities within that pick layer, sorted by depth.
type LayerMap = BTreeMap<PickLayer, DepthSortedHits>;

/// Maps Pointers to a [`LayerMap`]. Note this is much more complex than the [`HoverMap`] because
/// this data structure is used to sort entities by layer then depth for every pointer.
type OverMap = HashMap<PointerId, LayerMap>;

/// The source of truth for all hover state. This is used to determine what events to send, and what
/// state components should be in.
///
/// Maps pointers to the entities they are hovering over.
///
/// "Hovering" refers to the *hover* state, which is not the same as whether or not a picking
/// backend is reporting hits between a pointer and an entity. A pointer is "hovering" an entity
/// only if the pointer is hitting the entity (as reported by a picking backend) *and* no entities
/// between it and the pointer block interactions.
///
/// For example, if a pointer is hitting a UI button and a 3d mesh, but the button is in front of
/// the mesh, the UI button will be hovered, but the mesh will not. Unless, the [`Pickable`]
/// component is present with [`should_block_lower`](Pickable::should_block_lower) set to `false`.
///
/// # Advanced Users
///
/// If you want to completely replace the provided picking events or state produced by this plugin,
/// you can use this resource to do that. All of the event systems for picking are built *on top of*
/// this authoritative hover state, and you can do the same. You can also use the
/// [`PreviousHoverMap`] as a robust way of determining changes in hover state from the previous
/// update.
#[derive(Debug, Deref, DerefMut, Default, Resource)]
pub struct HoverMap(pub HashMap<PointerId, HashMap<Entity, HitData>>);

/// The previous state of the hover map, used to track changes to hover state.
#[derive(Debug, Deref, DerefMut, Default, Resource)]
pub struct PreviousHoverMap(pub HashMap<PointerId, HashMap<Entity, HitData>>);

/// Coalesces all data from inputs and backends to generate a map of the currently hovered entities.
/// This is the final focusing step to determine which entity the pointer is hovering over.
pub fn generate_hovermap(
    // Inputs
    pickable: Query<&Pickable>,
    pointers: Query<&PointerId>,
    mut under_pointer: EventReader<backend::PointerHits>,
    mut pointer_input: EventReader<PointerInput>,
    // Local
    mut over_map: Local<OverMap>,
    // Output
    mut hover_map: ResMut<HoverMap>,
    mut previous_hover_map: ResMut<PreviousHoverMap>,
) {
    reset_maps(
        &mut hover_map,
        &mut previous_hover_map,
        &mut over_map,
        &pointers,
    );
    build_over_map(&mut under_pointer, &mut over_map, &mut pointer_input);
    build_hover_map(&pointers, pickable, &over_map, &mut hover_map);
}

/// Clear non-empty local maps, reusing allocated memory.
fn reset_maps(
    hover_map: &mut HoverMap,
    previous_hover_map: &mut PreviousHoverMap,
    over_map: &mut OverMap,
    pointers: &Query<&PointerId>,
) {
    // Swap the previous and current hover maps. This results in the previous values being stored in
    // `PreviousHoverMap`. Swapping is okay because we clear the `HoverMap` which now holds stale
    // data. This process is done without any allocations.
    core::mem::swap(&mut previous_hover_map.0, &mut hover_map.0);

    for entity_set in hover_map.values_mut() {
        entity_set.clear();
    }
    for layer_map in over_map.values_mut() {
        layer_map.clear();
    }

    // Clear pointers from the maps if they have been removed.
    let active_pointers: Vec<PointerId> = pointers.iter().copied().collect();
    hover_map.retain(|pointer, _| active_pointers.contains(pointer));
    over_map.retain(|pointer, _| active_pointers.contains(pointer));
}

/// Build an ordered map of entities that are under each pointer
fn build_over_map(
    backend_events: &mut EventReader<backend::PointerHits>,
    pointer_over_map: &mut Local<OverMap>,
    pointer_input: &mut EventReader<PointerInput>,
) {
    let cancelled_pointers: HashSet<PointerId> = pointer_input
        .read()
        .filter_map(|p| {
            if let PointerAction::Cancel = p.action {
                Some(p.pointer_id)
            } else {
                None
            }
        })
        .collect();

    for entities_under_pointer in backend_events
        .read()
        .filter(|e| !cancelled_pointers.contains(&e.pointer))
    {
        let pointer = entities_under_pointer.pointer;
        let layer_map = pointer_over_map.entry(pointer).or_default();
        for (entity, pick_data) in entities_under_pointer.picks.iter() {
            let layer = entities_under_pointer.order;
            let hits = layer_map.entry(FloatOrd(layer)).or_default();
            hits.push((*entity, pick_data.clone()));
        }
    }

    for layers in pointer_over_map.values_mut() {
        for hits in layers.values_mut() {
            hits.sort_by_key(|(_, hit)| FloatOrd(hit.depth));
        }
    }
}

/// Build an unsorted set of hovered entities, accounting for depth, layer, and [`Pickable`]. Note
/// that unlike the pointer map, this uses [`Pickable`] to determine if lower entities receive hover
/// focus. Often, only a single entity per pointer will be hovered.
fn build_hover_map(
    pointers: &Query<&PointerId>,
    pickable: Query<&Pickable>,
    over_map: &Local<OverMap>,
    // Output
    hover_map: &mut HoverMap,
) {
    for pointer_id in pointers.iter() {
        let pointer_entity_set = hover_map.entry(*pointer_id).or_default();
        if let Some(layer_map) = over_map.get(pointer_id) {
            // Note we reverse here to start from the highest layer first.
            for (entity, pick_data) in layer_map.values().rev().flatten() {
                if let Ok(pickable) = pickable.get(*entity) {
                    if pickable.is_hoverable {
                        pointer_entity_set.insert(*entity, pick_data.clone());
                    }
                    if pickable.should_block_lower {
                        break;
                    }
                } else {
                    pointer_entity_set.insert(*entity, pick_data.clone()); // Emit events by default
                    break; // Entities block by default so we break out of the loop
                }
            }
        }
    }
}

/// A component that aggregates picking interaction state of this entity across all pointers.
///
/// Unlike bevy's `Interaction` component, this is an aggregate of the state of all pointers
/// interacting with this entity. Aggregation is done by taking the interaction with the highest
/// precedence.
///
/// For example, if we have an entity that is being hovered by one pointer, and pressed by another,
/// the entity will be considered pressed. If that entity is instead being hovered by both pointers,
/// it will be considered hovered.
#[derive(Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
pub enum PickingInteraction {
    /// The entity is being pressed down by a pointer.
    Pressed = 2,
    /// The entity is being hovered by a pointer.
    Hovered = 1,
    /// No pointers are interacting with this entity.
    #[default]
    None = 0,
}

/// Uses [`HoverMap`] changes to update [`PointerInteraction`] and [`PickingInteraction`] components.
pub fn update_interactions(
    // Input
    hover_map: Res<HoverMap>,
    previous_hover_map: Res<PreviousHoverMap>,
    // Outputs
    mut commands: Commands,
    mut pointers: Query<(&PointerId, &PointerPress, &mut PointerInteraction)>,
    mut interact: Query<&mut PickingInteraction>,
) {
    // Create a map to hold the aggregated interaction for each entity. This is needed because we
    // need to be able to insert the interaction component on entities if they do not exist. To do
    // so we need to know the final aggregated interaction state to avoid the scenario where we set
    // an entity to `Pressed`, then overwrite that with a lower precedent like `Hovered`.
    let mut new_interaction_state = HashMap::<Entity, PickingInteraction>::default();
    for (pointer, pointer_press, mut pointer_interaction) in &mut pointers {
        if let Some(pointers_hovered_entities) = hover_map.get(pointer) {
            // Insert a sorted list of hit entities into the pointer's interaction component.
            let mut sorted_entities: Vec<_> = pointers_hovered_entities.clone().drain().collect();
            sorted_entities.sort_by_key(|(_, hit)| FloatOrd(hit.depth));
            pointer_interaction.sorted_entities = sorted_entities;

            for hovered_entity in pointers_hovered_entities.iter().map(|(entity, _)| entity) {
                merge_interaction_states(pointer_press, hovered_entity, &mut new_interaction_state);
            }
        }
    }

    // Take the aggregated entity states and update or insert the component if missing.
    for (&hovered_entity, &new_interaction) in new_interaction_state.iter() {
        if let Ok(mut interaction) = interact.get_mut(hovered_entity) {
            interaction.set_if_neq(new_interaction);
        } else if let Ok(mut entity_commands) = commands.get_entity(hovered_entity) {
            entity_commands.try_insert(new_interaction);
        }
    }

    // Clear all previous hover data from pointers that are no longer hovering any entities.
    // We do this last to preserve change detection for picking interactions.
    for (pointer, _, _) in &mut pointers {
        let Some(previously_hovered_entities) = previous_hover_map.get(pointer) else {
            continue;
        };

        for entity in previously_hovered_entities.keys() {
            if !new_interaction_state.contains_key(entity)
                && let Ok(mut interaction) = interact.get_mut(*entity)
            {
                interaction.set_if_neq(PickingInteraction::None);
            }
        }
    }
}

/// Merge the interaction state of this entity into the aggregated map.
fn merge_interaction_states(
    pointer_press: &PointerPress,
    hovered_entity: &Entity,
    new_interaction_state: &mut HashMap<Entity, PickingInteraction>,
) {
    let new_interaction = match pointer_press.is_any_pressed() {
        true => PickingInteraction::Pressed,
        false => PickingInteraction::Hovered,
    };

    if let Some(old_interaction) = new_interaction_state.get_mut(hovered_entity) {
        // Only update if the new value has a higher precedence than the old value.
        if *old_interaction != new_interaction
            && matches!(
                (*old_interaction, new_interaction),
                (PickingInteraction::Hovered, PickingInteraction::Pressed)
                    | (PickingInteraction::None, PickingInteraction::Pressed)
                    | (PickingInteraction::None, PickingInteraction::Hovered)
            )
        {
            *old_interaction = new_interaction;
        }
    } else {
        new_interaction_state.insert(*hovered_entity, new_interaction);
    }
}

/// A component that allows users to use regular Bevy change detection to determine when the pointer
/// enters or leaves an entity. Users should insert this component on an entity to indicate interest
/// in knowing about hover state changes.
///
/// The component's boolean value will be `true` whenever the pointer is currently directly hovering
/// over the entity, or any of the entity's descendants (as defined by the [`ChildOf`]
/// relationship). This is consistent with the behavior of the CSS `:hover` pseudo-class, which
/// applies to the element and all of its descendants.
///
/// The contained boolean value is guaranteed to only be mutated when the pointer enters or leaves
/// the entity, allowing Bevy change detection to be used efficiently. This is in contrast to the
/// [`HoverMap`] resource, which is updated every frame.
///
/// Typically, a simple hoverable entity or widget will have this component added to it. More
/// complex widgets can have this component added to each hoverable part.
///
/// The computational cost of keeping the `Hovered` components up to date is relatively cheap, and
/// linear in the number of entities that have the [`Hovered`] component inserted.
#[derive(Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
#[component(immutable)]
pub struct Hovered(pub bool);

impl Hovered {
    /// Get whether the entity is currently hovered.
    pub fn get(&self) -> bool {
        self.0
    }
}

/// A component that allows users to use regular Bevy change detection to determine when the pointer
/// is directly hovering over an entity. Users should insert this component on an entity to indicate
/// interest in knowing about hover state changes.
///
/// This is similar to [`Hovered`] component, except that it does not include descendants in the
/// hover state.
#[derive(Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
#[component(immutable)]
pub struct DirectlyHovered(pub bool);

impl DirectlyHovered {
    /// Get whether the entity is currently hovered.
    pub fn get(&self) -> bool {
        self.0
    }
}

/// Uses [`HoverMap`] changes to update [`Hovered`] components.
pub fn update_is_hovered(
    hover_map: Option<Res<HoverMap>>,
    mut hovers: Query<(Entity, &Hovered)>,
    parent_query: Query<&ChildOf>,
    mut commands: Commands,
) {
    // Don't do any work if there's no hover map.
    let Some(hover_map) = hover_map else { return };

    // Don't bother collecting ancestors if there are no hovers.
    if hovers.is_empty() {
        return;
    }

    // Algorithm: for each entity having a `Hovered` component, we want to know if the current
    // entry in the hover map is "within" (that is, in the set of descendants of) that entity. Rather
    // than doing an expensive breadth-first traversal of children, instead start with the hovermap
    // entry and search upwards. We can make this even cheaper by building a set of ancestors for
    // the hovermap entry, and then testing each `Hovered` entity against that set.

    // A set which contains the hovered for the current pointer entity and its ancestors. The
    // capacity is based on the likely tree depth of the hierarchy, which is typically greater for
    // UI (because of layout issues) than for 3D scenes. A depth of 32 is a reasonable upper bound
    // for most use cases.
    let mut hover_ancestors = EntityHashSet::with_capacity(32);
    if let Some(map) = hover_map.get(&PointerId::Mouse) {
        for hovered_entity in map.keys() {
            hover_ancestors.insert(*hovered_entity);
            hover_ancestors.extend(parent_query.iter_ancestors(*hovered_entity));
        }
    }

    // For each hovered entity, it is considered "hovering" if it's in the set of hovered ancestors.
    for (entity, hoverable) in hovers.iter_mut() {
        let is_hovering = hover_ancestors.contains(&entity);
        if hoverable.0 != is_hovering {
            commands.entity(entity).insert(Hovered(is_hovering));
        }
    }
}

/// Uses [`HoverMap`] changes to update [`DirectlyHovered`] components.
pub fn update_is_directly_hovered(
    hover_map: Option<Res<HoverMap>>,
    hovers: Query<(Entity, &DirectlyHovered)>,
    mut commands: Commands,
) {
    // Don't do any work if there's no hover map.
    let Some(hover_map) = hover_map else { return };

    // Don't bother collecting ancestors if there are no hovers.
    if hovers.is_empty() {
        return;
    }

    if let Some(map) = hover_map.get(&PointerId::Mouse) {
        // It's hovering if it's in the HoverMap.
        for (entity, hoverable) in hovers.iter() {
            let is_hovering = map.contains_key(&entity);
            if hoverable.0 != is_hovering {
                commands.entity(entity).insert(DirectlyHovered(is_hovering));
            }
        }
    } else {
        // No hovered entity, reset all hovers.
        for (entity, hoverable) in hovers.iter() {
            if hoverable.0 {
                commands.entity(entity).insert(DirectlyHovered(false));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_camera::Camera;

    use super::*;

    #[test]
    fn update_is_hovered_memoized() {
        let mut world = World::default();
        let camera = world.spawn(Camera::default()).id();

        // Setup entities
        let hovered_child = world.spawn_empty().id();
        let hovered_entity = world.spawn(Hovered(false)).add_child(hovered_child).id();

        // Setup hover map with hovered_entity hovered by mouse
        let mut hover_map = HoverMap::default();
        let mut entity_map = HashMap::new();
        entity_map.insert(
            hovered_child,
            HitData {
                depth: 0.0,
                camera,
                position: None,
                normal: None,
            },
        );
        hover_map.insert(PointerId::Mouse, entity_map);
        world.insert_resource(hover_map);

        // Run the system
        assert!(world.run_system_cached(update_is_hovered).is_ok());

        // Check to insure that the hovered entity has the Hovered component set to true
        let hover = world.entity(hovered_entity).get_ref::<Hovered>().unwrap();
        assert!(hover.get());
        assert!(hover.is_changed());

        // Now do it again, but don't change the hover map.
        world.increment_change_tick();

        assert!(world.run_system_cached(update_is_hovered).is_ok());
        let hover = world.entity(hovered_entity).get_ref::<Hovered>().unwrap();
        assert!(hover.get());

        // Should not be changed
        // NOTE: Test doesn't work - thinks it is always changed
        // assert!(!hover.is_changed());

        // Clear the hover map and run again.
        world.insert_resource(HoverMap::default());
        world.increment_change_tick();

        assert!(world.run_system_cached(update_is_hovered).is_ok());
        let hover = world.entity(hovered_entity).get_ref::<Hovered>().unwrap();
        assert!(!hover.get());
        assert!(hover.is_changed());
    }

    #[test]
    fn update_is_hovered_direct_self() {
        let mut world = World::default();
        let camera = world.spawn(Camera::default()).id();

        // Setup entities
        let hovered_entity = world.spawn(DirectlyHovered(false)).id();

        // Setup hover map with hovered_entity hovered by mouse
        let mut hover_map = HoverMap::default();
        let mut entity_map = HashMap::new();
        entity_map.insert(
            hovered_entity,
            HitData {
                depth: 0.0,
                camera,
                position: None,
                normal: None,
            },
        );
        hover_map.insert(PointerId::Mouse, entity_map);
        world.insert_resource(hover_map);

        // Run the system
        assert!(world.run_system_cached(update_is_directly_hovered).is_ok());

        // Check to insure that the hovered entity has the DirectlyHovered component set to true
        let hover = world
            .entity(hovered_entity)
            .get_ref::<DirectlyHovered>()
            .unwrap();
        assert!(hover.get());
        assert!(hover.is_changed());

        // Now do it again, but don't change the hover map.
        world.increment_change_tick();

        assert!(world.run_system_cached(update_is_directly_hovered).is_ok());
        let hover = world
            .entity(hovered_entity)
            .get_ref::<DirectlyHovered>()
            .unwrap();
        assert!(hover.get());

        // Should not be changed
        // NOTE: Test doesn't work - thinks it is always changed
        // assert!(!hover.is_changed());

        // Clear the hover map and run again.
        world.insert_resource(HoverMap::default());
        world.increment_change_tick();

        assert!(world.run_system_cached(update_is_directly_hovered).is_ok());
        let hover = world
            .entity(hovered_entity)
            .get_ref::<DirectlyHovered>()
            .unwrap();
        assert!(!hover.get());
        assert!(hover.is_changed());
    }

    #[test]
    fn update_is_hovered_direct_child() {
        let mut world = World::default();
        let camera = world.spawn(Camera::default()).id();

        // Setup entities
        let hovered_child = world.spawn_empty().id();
        let hovered_entity = world
            .spawn(DirectlyHovered(false))
            .add_child(hovered_child)
            .id();

        // Setup hover map with hovered_entity hovered by mouse
        let mut hover_map = HoverMap::default();
        let mut entity_map = HashMap::new();
        entity_map.insert(
            hovered_child,
            HitData {
                depth: 0.0,
                camera,
                position: None,
                normal: None,
            },
        );
        hover_map.insert(PointerId::Mouse, entity_map);
        world.insert_resource(hover_map);

        // Run the system
        assert!(world.run_system_cached(update_is_directly_hovered).is_ok());

        // Check to insure that the DirectlyHovered component is still false
        let hover = world
            .entity(hovered_entity)
            .get_ref::<DirectlyHovered>()
            .unwrap();
        assert!(!hover.get());
        assert!(hover.is_changed());
    }
}
