use alloc::{boxed::Box, vec::Vec};
use core::ops::{Deref, DerefMut, Index, IndexMut, Range};

use bevy_platform::collections::HashMap;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::{
    component::ComponentId,
    prelude::SystemSet,
    query::FilteredAccessSet,
    schedule::{BoxedCondition, InternedSystemSet},
    system::{ReadOnlySystem, ScheduleSystem},
    world::World,
};

/// A [`SystemWithAccess`] stored in a [`ScheduleGraph`](crate::schedule::ScheduleGraph).
pub(crate) struct SystemNode {
    pub(crate) inner: Option<SystemWithAccess>,
}

/// A [`ScheduleSystem`] stored alongside the access returned from [`System::initialize`](crate::system::System::initialize).
pub struct SystemWithAccess {
    /// The system itself.
    pub system: ScheduleSystem,
    /// The access returned by [`System::initialize`](crate::system::System::initialize).
    /// This will be empty if the system has not been initialized yet.
    pub access: FilteredAccessSet<ComponentId>,
}

impl SystemWithAccess {
    /// Constructs a new [`SystemWithAccess`] from a [`ScheduleSystem`].
    /// The `access` will initially be empty.
    pub fn new(system: ScheduleSystem) -> Self {
        Self {
            system,
            access: FilteredAccessSet::new(),
        }
    }
}

impl Deref for SystemWithAccess {
    type Target = ScheduleSystem;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl DerefMut for SystemWithAccess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.system
    }
}

/// A [`BoxedCondition`] stored alongside the access returned from [`System::initialize`](crate::system::System::initialize).
pub struct ConditionWithAccess {
    /// The condition itself.
    pub condition: BoxedCondition,
    /// The access returned by [`System::initialize`](crate::system::System::initialize).
    /// This will be empty if the system has not been initialized yet.
    pub access: FilteredAccessSet<ComponentId>,
}

impl ConditionWithAccess {
    /// Constructs a new [`ConditionWithAccess`] from a [`BoxedCondition`].
    /// The `access` will initially be empty.
    pub const fn new(condition: BoxedCondition) -> Self {
        Self {
            condition,
            access: FilteredAccessSet::new(),
        }
    }
}

impl Deref for ConditionWithAccess {
    type Target = BoxedCondition;

    fn deref(&self) -> &Self::Target {
        &self.condition
    }
}

impl DerefMut for ConditionWithAccess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.condition
    }
}

impl SystemNode {
    /// Create a new [`SystemNode`]
    pub fn new(system: ScheduleSystem) -> Self {
        Self {
            inner: Some(SystemWithAccess::new(system)),
        }
    }

    /// Obtain a reference to the [`SystemWithAccess`] represented by this node.
    pub fn get(&self) -> Option<&SystemWithAccess> {
        self.inner.as_ref()
    }

    /// Obtain a mutable reference to the [`SystemWithAccess`] represented by this node.
    pub fn get_mut(&mut self) -> Option<&mut SystemWithAccess> {
        self.inner.as_mut()
    }
}

new_key_type! {
    /// A unique identifier for a system in a [`ScheduleGraph`].
    pub struct SystemKey;
    /// A unique identifier for a system set in a [`ScheduleGraph`].
    pub struct SystemSetKey;
}

/// Container for systems in a schedule.
#[derive(Default)]
pub struct Systems {
    /// List of systems in the schedule.
    nodes: SlotMap<SystemKey, SystemNode>,
    /// List of conditions for each system, in the same order as `nodes`.
    conditions: SecondaryMap<SystemKey, Vec<ConditionWithAccess>>,
    /// Systems and their conditions that have not been initialized yet.
    uninit: Vec<SystemKey>,
}

impl Systems {
    /// Returns the number of systems in this container.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if this container is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns a reference to the system with the given key, if it exists.
    pub fn get(&self, key: SystemKey) -> Option<&SystemWithAccess> {
        self.nodes.get(key).and_then(|node| node.get())
    }

    /// Returns a mutable reference to the system with the given key, if it exists.
    pub fn get_mut(&mut self, key: SystemKey) -> Option<&mut SystemWithAccess> {
        self.nodes.get_mut(key).and_then(|node| node.get_mut())
    }

    /// Returns a mutable reference to the system with the given key, panicking
    /// if it does not exist.
    ///
    /// # Panics
    ///
    /// If the system with the given key does not exist in this container.
    pub(crate) fn node_mut(&mut self, key: SystemKey) -> &mut SystemNode {
        self.nodes
            .get_mut(key)
            .unwrap_or_else(|| panic!("System with key {:?} does not exist in the schedule", key))
    }

    /// Returns `true` if the system with the given key has conditions.
    pub fn has_conditions(&self, key: SystemKey) -> bool {
        self.conditions
            .get(key)
            .is_some_and(|conditions| !conditions.is_empty())
    }

    /// Returns a reference to the conditions for the system with the given key, if it exists.
    pub fn get_conditions(&self, key: SystemKey) -> Option<&[ConditionWithAccess]> {
        self.conditions.get(key).map(Vec::as_slice)
    }

    /// Returns a mutable reference to the conditions for the system with the given key, if it exists.
    pub fn get_conditions_mut(&mut self, key: SystemKey) -> Option<&mut Vec<ConditionWithAccess>> {
        self.conditions.get_mut(key)
    }

    /// Returns an iterator over all systems and their conditions in this
    /// container.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (SystemKey, &ScheduleSystem, &[ConditionWithAccess])> + '_ {
        self.nodes.iter().filter_map(|(key, node)| {
            let system = &node.get()?.system;
            let conditions = self
                .conditions
                .get(key)
                .map(Vec::as_slice)
                .unwrap_or_default();
            Some((key, system, conditions))
        })
    }

    /// Inserts a new system into the container, along with its conditions,
    /// and queues it to be initialized later in [`Systems::initialize`].
    ///
    /// We have to defer initialization of systems in the container until we have
    /// `&mut World` access, so we store these in a list until
    /// [`Systems::initialize`] is called. This is usually done upon the first
    /// run of the schedule.
    pub fn insert(
        &mut self,
        system: ScheduleSystem,
        conditions: Vec<Box<dyn ReadOnlySystem<In = (), Out = bool>>>,
    ) -> SystemKey {
        let key = self.nodes.insert(SystemNode::new(system));
        self.conditions.insert(
            key,
            conditions
                .into_iter()
                .map(ConditionWithAccess::new)
                .collect(),
        );
        self.uninit.push(key);
        key
    }

    /// Returns `true` if all systems in this container have been initialized.
    pub fn is_initialized(&self) -> bool {
        self.uninit.is_empty()
    }

    /// Initializes all systems and their conditions that have not been
    /// initialized yet.
    pub fn initialize(&mut self, world: &mut World) {
        for key in self.uninit.drain(..) {
            let Some(system) = self.nodes.get_mut(key).and_then(|node| node.get_mut()) else {
                continue;
            };
            system.access = system.system.initialize(world);
            let Some(conditions) = self.conditions.get_mut(key) else {
                continue;
            };
            for condition in conditions {
                condition.access = condition.condition.initialize(world);
            }
        }
    }
}

impl Index<SystemKey> for Systems {
    type Output = SystemWithAccess;

    #[track_caller]
    fn index(&self, key: SystemKey) -> &Self::Output {
        self.get(key)
            .unwrap_or_else(|| panic!("System with key {:?} does not exist in the schedule", key))
    }
}

impl IndexMut<SystemKey> for Systems {
    #[track_caller]
    fn index_mut(&mut self, key: SystemKey) -> &mut Self::Output {
        self.get_mut(key)
            .unwrap_or_else(|| panic!("System with key {:?} does not exist in the schedule", key))
    }
}

/// Container for system sets in a schedule.
#[derive(Default)]
pub struct SystemSets {
    /// List of system sets in the schedule.
    sets: SlotMap<SystemSetKey, InternedSystemSet>,
    /// List of conditions for each system set, in the same order as `sets`.
    conditions: SecondaryMap<SystemSetKey, Vec<ConditionWithAccess>>,
    /// Map from system sets to their keys.
    ids: HashMap<InternedSystemSet, SystemSetKey>,
    /// System sets that have not been initialized yet.
    uninit: Vec<UninitializedSet>,
}

/// A system set's conditions that have not been initialized yet.
struct UninitializedSet {
    key: SystemSetKey,
    /// The range of indices in [`SystemSets::conditions`] that correspond
    /// to conditions that have not been initialized yet.
    ///
    /// [`SystemSets::conditions`] for a given set may be appended to
    /// multiple times (e.g. when `configure_sets` is called multiple with
    /// the same set), so we need to track which conditions in that list
    /// are newly added and not yet initialized.
    ///
    /// Systems don't need this tracking because each `add_systems` call
    /// creates separate nodes in the graph with their own conditions,
    /// so all conditions are initialized together.
    uninitialized_conditions: Range<usize>,
}

impl SystemSets {
    /// Returns the number of system sets in this container.
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    /// Returns `true` if this container is empty.
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// Returns `true` if the given set is present in this container.
    pub fn contains(&self, set: impl SystemSet) -> bool {
        self.ids.contains_key(&set.intern())
    }

    /// Returns a reference to the system set with the given key, if it exists.
    pub fn get(&self, key: SystemSetKey) -> Option<&dyn SystemSet> {
        self.sets.get(key).map(|set| &**set)
    }

    /// Returns the key for the given system set, inserting it into this
    /// container if it does not already exist.
    pub fn get_key_or_insert(&mut self, set: InternedSystemSet) -> SystemSetKey {
        *self.ids.entry(set).or_insert_with(|| {
            let key = self.sets.insert(set);
            self.conditions.insert(key, Vec::new());
            key
        })
    }

    /// Returns `true` if the system set with the given key has conditions.
    pub fn has_conditions(&self, key: SystemSetKey) -> bool {
        self.conditions
            .get(key)
            .is_some_and(|conditions| !conditions.is_empty())
    }

    /// Returns a reference to the conditions for the system set with the given
    /// key, if it exists.
    pub fn get_conditions(&self, key: SystemSetKey) -> Option<&[ConditionWithAccess]> {
        self.conditions.get(key).map(Vec::as_slice)
    }

    /// Returns a mutable reference to the conditions for the system set with
    /// the given key, if it exists.
    pub fn get_conditions_mut(
        &mut self,
        key: SystemSetKey,
    ) -> Option<&mut Vec<ConditionWithAccess>> {
        self.conditions.get_mut(key)
    }

    /// Returns an iterator over all system sets in this container, along with
    /// their conditions.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (SystemSetKey, &dyn SystemSet, &[ConditionWithAccess])> {
        self.sets.iter().filter_map(|(key, set)| {
            let conditions = self.conditions.get(key)?.as_slice();
            Some((key, &**set, conditions))
        })
    }

    /// Inserts conditions for a system set into the container, and queues the
    /// newly added conditions to be initialized later in [`SystemSets::initialize`].
    ///
    /// If the set was not already present in the container, it is added automatically.
    ///
    /// We have to defer initialization of system set conditions in the container
    /// until we have `&mut World` access, so we store these in a list until
    /// [`SystemSets::initialize`] is called. This is usually done upon the
    /// first run of the schedule.
    pub fn insert(
        &mut self,
        set: InternedSystemSet,
        new_conditions: Vec<Box<dyn ReadOnlySystem<In = (), Out = bool>>>,
    ) -> SystemSetKey {
        let key = self.get_key_or_insert(set);
        if !new_conditions.is_empty() {
            let current_conditions = &mut self.conditions[key];
            let start = current_conditions.len();
            self.uninit.push(UninitializedSet {
                key,
                uninitialized_conditions: start..(start + new_conditions.len()),
            });
            current_conditions.extend(new_conditions.into_iter().map(ConditionWithAccess::new));
        }
        key
    }

    /// Returns `true` if all system sets' conditions in this container have
    /// been initialized.
    pub fn is_initialized(&self) -> bool {
        self.uninit.is_empty()
    }

    /// Initializes all system sets' conditions that have not been
    /// initialized yet. Because a system set's conditions may be appended to
    /// multiple times, we track which conditions were added since the last
    /// initialization and only initialize those.
    pub fn initialize(&mut self, world: &mut World) {
        for uninit in self.uninit.drain(..) {
            let Some(conditions) = self.conditions.get_mut(uninit.key) else {
                continue;
            };
            for condition in &mut conditions[uninit.uninitialized_conditions] {
                condition.access = condition.initialize(world);
            }
        }
    }
}

impl Index<SystemSetKey> for SystemSets {
    type Output = dyn SystemSet;

    #[track_caller]
    fn index(&self, key: SystemSetKey) -> &Self::Output {
        self.get(key).unwrap_or_else(|| {
            panic!(
                "System set with key {:?} does not exist in the schedule",
                key
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec};

    use crate::{
        prelude::SystemSet,
        schedule::{SystemSets, Systems},
        system::IntoSystem,
        world::World,
    };

    #[derive(SystemSet, Clone, Copy, PartialEq, Eq, Debug, Hash)]
    pub struct TestSet;

    #[test]
    fn systems() {
        fn empty_system() {}

        let mut systems = Systems::default();
        assert!(systems.is_empty());
        assert_eq!(systems.len(), 0);

        let system = Box::new(IntoSystem::into_system(empty_system));
        let key = systems.insert(system, vec![]);

        assert!(!systems.is_empty());
        assert_eq!(systems.len(), 1);
        assert!(systems.get(key).is_some());
        assert!(systems.get_conditions(key).is_some());
        assert!(systems.get_conditions(key).unwrap().is_empty());
        assert!(systems.get_mut(key).is_some());
        assert!(!systems.is_initialized());
        assert!(systems.iter().next().is_some());

        let mut world = World::new();
        systems.initialize(&mut world);
        assert!(systems.is_initialized());
    }

    #[test]
    fn system_sets() {
        fn always_true() -> bool {
            true
        }

        let mut sets = SystemSets::default();
        assert!(sets.is_empty());
        assert_eq!(sets.len(), 0);

        let condition = Box::new(IntoSystem::into_system(always_true));
        let key = sets.insert(TestSet.intern(), vec![condition]);

        assert!(!sets.is_empty());
        assert_eq!(sets.len(), 1);
        assert!(sets.get(key).is_some());
        assert!(sets.get_conditions(key).is_some());
        assert!(!sets.get_conditions(key).unwrap().is_empty());
        assert!(!sets.is_initialized());
        assert!(sets.iter().next().is_some());

        let mut world = World::new();
        sets.initialize(&mut world);
        assert!(sets.is_initialized());
    }
}
