//! Centralized storage for observers, allowing for efficient look-ups.
//!
//! This has multiple levels:
//! - [`World::observers`](crate::world::World::observers) provides access to [`Observers`], which is a central storage for all observers.
//! - [`Observers`] contains multiple distinct caches in the form of [`CachedObservers`].
//!     - Most observers are looked up by the [`ComponentId`] of the event they are observing
//!     - Lifecycle observers have their own fields to save lookups.
//! - [`CachedObservers`] contains maps of [`ObserverRunner`]s, which are the actual functions that will be run when the observer is triggered.
//!     - These are split by target type, in order to allow for different lookup strategies.
//!     - [`CachedComponentObservers`] is one of these maps, which contains observers that are specifically targeted at a component.

use alloc::{vec, vec::Vec};
use bevy_platform::collections::HashMap;
use smallvec::SmallVec;

use crate::{
    archetype::ArchetypeFlags,
    component::ComponentId,
    entity::{Entity, EntityHashMap},
    event::EventKey,
    intern::Interned,
    observer::{EdgeTarget, ObserverDescriptor, ObserverRunner, ObserverSet},
};

/// An internal lookup table tracking all of the observers in the world.
///
/// Stores a cache mapping event ids to their registered observers.
/// Some observer kinds (like [lifecycle](crate::lifecycle) observers) have a dedicated field,
/// saving lookups for the most common triggers.
///
/// This can be accessed via [`World::observers`](crate::world::World::observers).
#[derive(Default, Debug)]
pub struct Observers {
    // Cached ECS observers to save a lookup for high-traffic built-in event types.
    add: CachedObservers,
    insert: CachedObservers,
    discard: CachedObservers,
    remove: CachedObservers,
    despawn: CachedObservers,
    // Map from event type to set of observers watching for that event
    cache: HashMap<EventKey, CachedObservers>,
    // Observer set hierarchy declarations. Populated by observer set configuration in a later phase.
    #[expect(dead_code, reason = "Observer set dispatch is wired in a later phase.")]
    set_hierarchy: HashMap<Interned<dyn ObserverSet>, SmallVec<[Interned<dyn ObserverSet>; 2]>>,
}

impl Observers {
    pub(crate) fn get_observers_mut(&mut self, event_key: EventKey) -> &mut CachedObservers {
        use crate::lifecycle::*;

        match event_key {
            ADD => &mut self.add,
            INSERT => &mut self.insert,
            DISCARD => &mut self.discard,
            REMOVE => &mut self.remove,
            DESPAWN => &mut self.despawn,
            _ => self.cache.entry(event_key).or_default(),
        }
    }

    /// Attempts to get the observers for the given `event_key`.
    ///
    /// When accessing the observers for lifecycle events, such as [`Add`], [`Insert`], [`Discard`], [`Remove`], and [`Despawn`],
    /// use the [`EventKey`] constants from the [`lifecycle`](crate::lifecycle) module.
    ///
    /// [`Add`]: crate::lifecycle::Add
    /// [`Insert`]: crate::lifecycle::Insert
    /// [`Discard`]: crate::lifecycle::Discard
    /// [`Remove`]: crate::lifecycle::Remove
    /// [`Despawn`]: crate::lifecycle::Despawn
    pub fn try_get_observers(&self, event_key: EventKey) -> Option<&CachedObservers> {
        use crate::lifecycle::*;

        match event_key {
            ADD => Some(&self.add),
            INSERT => Some(&self.insert),
            DISCARD => Some(&self.discard),
            REMOVE => Some(&self.remove),
            DESPAWN => Some(&self.despawn),
            _ => self.cache.get(&event_key),
        }
    }

    pub(crate) fn is_archetype_cached(event_key: EventKey) -> Option<ArchetypeFlags> {
        use crate::lifecycle::*;

        match event_key {
            ADD => Some(ArchetypeFlags::ON_ADD_OBSERVER),
            INSERT => Some(ArchetypeFlags::ON_INSERT_OBSERVER),
            DISCARD => Some(ArchetypeFlags::ON_DISCARD_OBSERVER),
            REMOVE => Some(ArchetypeFlags::ON_REMOVE_OBSERVER),
            DESPAWN => Some(ArchetypeFlags::ON_DESPAWN_OBSERVER),
            _ => None,
        }
    }

    pub(crate) fn update_archetype_flags(
        &self,
        component_id: ComponentId,
        flags: &mut ArchetypeFlags,
    ) {
        if self
            .add
            .component_observers_legacy
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_ADD_OBSERVER);
        }

        if self
            .insert
            .component_observers_legacy
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_INSERT_OBSERVER);
        }

        if self
            .discard
            .component_observers_legacy
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_DISCARD_OBSERVER);
        }

        if self
            .remove
            .component_observers_legacy
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_REMOVE_OBSERVER);
        }

        if self
            .despawn
            .component_observers_legacy
            .contains_key(&component_id)
        {
            flags.insert(ArchetypeFlags::ON_DESPAWN_OBSERVER);
        }
    }
}

/// Identifier for an observer node in [`CachedObservers`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u32);

impl NodeId {
    fn new(index: usize) -> Self {
        debug_assert!(u32::try_from(index).is_ok());
        Self(index as u32)
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

/// Observer data stored once per registered observer/event-key pair.
#[derive(Clone, Copy, Debug)]
pub struct ObserverNode {
    /// The entity that owns the observer component.
    pub observer: Entity,
    /// The function used to run the observer.
    pub runner: ObserverRunner,
    sort_key: u64,
}

/// An ordering edge stored in event-local observer storage.
#[derive(Clone, Debug)]
pub struct ObserverEdgeResolved {
    owner: Entity,
    #[expect(
        dead_code,
        reason = "Observer edge sorting is wired in the graph-sort phase."
    )]
    from: EdgeTarget,
    #[expect(
        dead_code,
        reason = "Observer edge sorting is wired in the graph-sort phase."
    )]
    to: EdgeTarget,
}

/// Per-component observer indices.
#[derive(Default, Debug)]
pub struct ComponentBucket {
    globals: SmallVec<[NodeId; 2]>,
    by_entity: EntityHashMap<SmallVec<[NodeId; 2]>>,
}

impl ComponentBucket {
    fn is_empty(&self) -> bool {
        self.globals.is_empty() && self.by_entity.is_empty()
    }
}

/// Collection of [`ObserverRunner`] for [`Observer`](crate::observer::Observer) registered to a particular event.
///
/// This is stored inside of [`Observers`], specialized for each kind of observer.
#[derive(Default, Debug)]
pub struct CachedObservers {
    /// Observers watching for any time this event is triggered, regardless of target.
    /// These will also respond to events targeting specific components or entities
    pub(super) global_observers_legacy: ObserverMap,
    /// Observers watching for triggers of events for a specific component
    pub(super) component_observers_legacy: HashMap<ComponentId, CachedComponentObservers>,
    /// Observers watching for triggers of events for a specific entity
    pub(super) entity_observers_legacy: EntityHashMap<ObserverMap>,
    nodes: Vec<ObserverNode>,
    order: Vec<NodeId>,
    globals: SmallVec<[NodeId; 4]>,
    by_entity: EntityHashMap<SmallVec<[NodeId; 2]>>,
    by_component: HashMap<ComponentId, ComponentBucket>,
    sets: HashMap<Interned<dyn ObserverSet>, SmallVec<[NodeId; 4]>>,
    edges: Vec<ObserverEdgeResolved>,
    observer_to_node: EntityHashMap<NodeId>,
    dirty: bool,
    next_sort_key: u64,
}

impl CachedObservers {
    /// Observers watching for any time this event is triggered, regardless of target.
    /// These will also respond to events targeting specific components or entities
    pub fn global_observers(&self) -> &ObserverMap {
        &self.global_observers_legacy
    }

    /// Returns observers watching for triggers of events for a specific component.
    pub fn component_observers(&self) -> &HashMap<ComponentId, CachedComponentObservers> {
        &self.component_observers_legacy
    }

    /// Returns observers watching for triggers of events for a specific entity.
    pub fn entity_observers(&self) -> &EntityHashMap<ObserverMap> {
        &self.entity_observers_legacy
    }

    pub(crate) fn register_observer(
        &mut self,
        observer: Entity,
        runner: ObserverRunner,
        descriptor: &ObserverDescriptor,
    ) -> SmallVec<[ComponentId; 4]> {
        if self.observer_to_node.contains_key(&observer) {
            self.unregister_observer(observer, descriptor);
        }

        let node_id = NodeId::new(self.nodes.len());
        self.nodes.push(ObserverNode {
            observer,
            runner,
            sort_key: self.next_sort_key,
        });
        self.next_sort_key = self.next_sort_key.wrapping_add(1);
        self.observer_to_node.insert(observer, node_id);

        if descriptor.components.is_empty() && descriptor.entities.is_empty() {
            push_unique(&mut self.globals, node_id);
        } else if descriptor.components.is_empty() {
            for &watched_entity in &descriptor.entities {
                push_unique(self.by_entity.entry(watched_entity).or_default(), node_id);
            }
        } else {
            for &component in &descriptor.components {
                let bucket = self.by_component.entry(component).or_default();
                if descriptor.entities.is_empty() {
                    push_unique(&mut bucket.globals, node_id);
                } else {
                    for &watched_entity in &descriptor.entities {
                        push_unique(bucket.by_entity.entry(watched_entity).or_default(), node_id);
                    }
                }
            }
        }

        for &set in &descriptor.sets {
            push_unique(self.sets.entry(set).or_default(), node_id);
        }

        for edge in &descriptor.edges {
            self.edges.push(ObserverEdgeResolved {
                owner: observer,
                from: edge.from.clone(),
                to: edge.to.clone(),
            });
        }

        self.dirty = true;
        self.resort();

        let mut newly_observed_components = SmallVec::new();
        for &component in &descriptor.components {
            if self.by_component.contains_key(&component) {
                push_unique_component(&mut newly_observed_components, component);
            }
        }
        newly_observed_components
    }

    pub(crate) fn unregister_observer(
        &mut self,
        observer: Entity,
        descriptor: &ObserverDescriptor,
    ) -> SmallVec<[ComponentId; 4]> {
        let Some(node_id) = self.observer_to_node.remove(&observer) else {
            return SmallVec::new();
        };

        remove_node_id(&mut self.globals, node_id);
        remove_node_id_from_entity_map(&mut self.by_entity, node_id, &descriptor.entities);

        let mut removed_components = SmallVec::new();
        for &component in &descriptor.components {
            let Some(bucket) = self.by_component.get_mut(&component) else {
                continue;
            };

            remove_node_id(&mut bucket.globals, node_id);
            remove_node_id_from_entity_map(&mut bucket.by_entity, node_id, &descriptor.entities);

            if bucket.is_empty() {
                self.by_component.remove(&component);
                push_unique_component(&mut removed_components, component);
            }
        }

        for set in &descriptor.sets {
            let Some(nodes) = self.sets.get_mut(set) else {
                continue;
            };
            remove_node_id(nodes, node_id);
            if nodes.is_empty() {
                self.sets.remove(set);
            }
        }

        self.edges.retain(|edge| edge.owner != observer);
        self.remove_node(node_id);
        self.dirty = true;
        self.resort();

        removed_components
    }

    #[expect(
        dead_code,
        reason = "The new component-observer check is used after the register/unregister cutover."
    )]
    pub(crate) fn contains_component_observers(&self, component_id: ComponentId) -> bool {
        self.by_component
            .get(&component_id)
            .is_some_and(|bucket| !bucket.is_empty())
    }

    pub(crate) fn resort(&mut self) {
        if !self.dirty {
            return;
        }

        self.order.clear();
        self.order.extend((0..self.nodes.len()).map(NodeId::new));
        self.order
            .sort_by_key(|node_id| self.nodes[node_id.index()].sort_key);
        self.sort_indices();
        self.dirty = false;

        #[cfg(debug_assertions)]
        self.debug_assert_sorted_indices();
    }

    fn remove_node(&mut self, node_id: NodeId) {
        let index = node_id.index();
        let last_index = self.nodes.len() - 1;
        self.nodes.swap_remove(index);

        if index != last_index {
            let moved_from = NodeId::new(last_index);
            let moved_to = node_id;
            self.observer_to_node
                .insert(self.nodes[index].observer, moved_to);
            self.replace_node_id(moved_from, moved_to);
        }
    }

    fn replace_node_id(&mut self, from: NodeId, to: NodeId) {
        replace_node_id(&mut self.order, from, to);
        replace_node_id(&mut self.globals, from, to);
        replace_node_id_in_entity_map(&mut self.by_entity, from, to);

        for bucket in self.by_component.values_mut() {
            replace_node_id(&mut bucket.globals, from, to);
            replace_node_id_in_entity_map(&mut bucket.by_entity, from, to);
        }

        for nodes in self.sets.values_mut() {
            replace_node_id(nodes, from, to);
        }
    }

    fn sort_indices(&mut self) {
        let mut positions = vec![usize::MAX; self.nodes.len()];
        for (position, node_id) in self.order.iter().copied().enumerate() {
            positions[node_id.index()] = position;
        }

        sort_node_ids(&positions, &mut self.globals);
        sort_entity_map_node_ids(&positions, &mut self.by_entity);

        for bucket in self.by_component.values_mut() {
            sort_node_ids(&positions, &mut bucket.globals);
            sort_entity_map_node_ids(&positions, &mut bucket.by_entity);
        }

        for nodes in self.sets.values_mut() {
            sort_node_ids(&positions, nodes);
        }
    }

    #[cfg(debug_assertions)]
    fn debug_assert_sorted_indices(&self) {
        let mut positions = vec![usize::MAX; self.nodes.len()];
        for (position, node_id) in self.order.iter().copied().enumerate() {
            positions[node_id.index()] = position;
        }

        debug_assert!(is_sorted_by_order(&positions, &self.globals));
        for nodes in self.by_entity.values() {
            debug_assert!(is_sorted_by_order(&positions, nodes));
        }
        for bucket in self.by_component.values() {
            debug_assert!(is_sorted_by_order(&positions, &bucket.globals));
            for nodes in bucket.by_entity.values() {
                debug_assert!(is_sorted_by_order(&positions, nodes));
            }
        }
        for nodes in self.sets.values() {
            debug_assert!(is_sorted_by_order(&positions, nodes));
        }
    }
}

/// Map between an observer entity and its [`ObserverRunner`]
pub type ObserverMap = EntityHashMap<ObserverRunner>;

/// Collection of [`ObserverRunner`] for [`Observer`](crate::observer::Observer) registered to a particular event targeted at a specific component.
///
/// This is stored inside of [`CachedObservers`].
#[derive(Default, Debug)]
pub struct CachedComponentObservers {
    // Observers watching for events targeting this component, but not a specific entity
    pub(super) global_observers: ObserverMap,
    // Observers watching for events targeting this component on a specific entity
    pub(super) entity_component_observers: EntityHashMap<ObserverMap>,
}

impl CachedComponentObservers {
    /// Returns observers watching for events targeting this component, but not a specific entity
    pub fn global_observers(&self) -> &ObserverMap {
        &self.global_observers
    }

    /// Returns observers watching for events targeting this component on a specific entity
    pub fn entity_component_observers(&self) -> &EntityHashMap<ObserverMap> {
        &self.entity_component_observers
    }
}

fn push_unique<const N: usize>(nodes: &mut SmallVec<[NodeId; N]>, node_id: NodeId) {
    if !nodes.contains(&node_id) {
        nodes.push(node_id);
    }
}

fn push_unique_component(components: &mut SmallVec<[ComponentId; 4]>, component_id: ComponentId) {
    if !components.contains(&component_id) {
        components.push(component_id);
    }
}

fn remove_node_id<const N: usize>(nodes: &mut SmallVec<[NodeId; N]>, node_id: NodeId) {
    nodes.retain(|id| *id != node_id);
}

fn remove_node_id_from_entity_map(
    by_entity: &mut EntityHashMap<SmallVec<[NodeId; 2]>>,
    node_id: NodeId,
    entities: &[Entity],
) {
    for entity in entities {
        let Some(nodes) = by_entity.get_mut(entity) else {
            continue;
        };
        remove_node_id(nodes, node_id);
        if nodes.is_empty() {
            by_entity.remove(entity);
        }
    }
}

fn replace_node_id(nodes: &mut [NodeId], from: NodeId, to: NodeId) {
    for node_id in nodes {
        if *node_id == from {
            *node_id = to;
        }
    }
}

fn replace_node_id_in_entity_map(
    by_entity: &mut EntityHashMap<SmallVec<[NodeId; 2]>>,
    from: NodeId,
    to: NodeId,
) {
    for nodes in by_entity.values_mut() {
        replace_node_id(nodes, from, to);
    }
}

fn sort_node_ids<const N: usize>(positions: &[usize], nodes: &mut SmallVec<[NodeId; N]>) {
    nodes.sort_by_key(|node_id| positions[node_id.index()]);
}

fn sort_entity_map_node_ids(
    positions: &[usize],
    by_entity: &mut EntityHashMap<SmallVec<[NodeId; 2]>>,
) {
    for nodes in by_entity.values_mut() {
        sort_node_ids(positions, nodes);
    }
}

#[cfg(debug_assertions)]
fn is_sorted_by_order<const N: usize>(positions: &[usize], nodes: &SmallVec<[NodeId; N]>) -> bool {
    nodes
        .windows(2)
        .all(|window| positions[window[0].index()] <= positions[window[1].index()])
}
