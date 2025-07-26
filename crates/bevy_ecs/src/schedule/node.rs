use alloc::{boxed::Box, vec::Vec};
use bevy_utils::prelude::DebugName;
use core::{
    any::TypeId,
    fmt::{self, Debug},
    ops::{Index, IndexMut, Range},
};

use bevy_platform::collections::HashMap;
use slotmap::{new_key_type, Key, KeyData, SecondaryMap, SlotMap};

use crate::{
    component::{CheckChangeTicks, Tick},
    prelude::{SystemIn, SystemSet},
    query::FilteredAccessSet,
    schedule::{
        graph::{Direction, GraphNodeId},
        BoxedCondition, InternedSystemSet,
    },
    system::{
        ReadOnlySystem, RunSystemError, ScheduleSystem, System, SystemParamValidationError,
        SystemStateFlags,
    },
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

/// A [`SystemWithAccess`] stored in a [`ScheduleGraph`](crate::schedule::ScheduleGraph).
pub(crate) struct SystemNode {
    pub(crate) inner: Option<SystemWithAccess>,
}

/// A [`ScheduleSystem`] stored alongside the access returned from [`System::initialize`].
pub struct SystemWithAccess {
    /// The system itself.
    pub system: ScheduleSystem,
    /// The access returned by [`System::initialize`].
    /// This will be empty if the system has not been initialized yet.
    pub access: FilteredAccessSet,
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

impl System for SystemWithAccess {
    type In = ();
    type Out = ();

    #[inline]
    fn name(&self) -> DebugName {
        self.system.name()
    }

    #[inline]
    fn type_id(&self) -> TypeId {
        self.system.type_id()
    }

    #[inline]
    fn flags(&self) -> SystemStateFlags {
        self.system.flags()
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        // SAFETY: Caller ensures the same safety requirements.
        unsafe { self.system.run_unsafe(input, world) }
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.system.refresh_hotpatch();
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.system.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.system.queue_deferred(world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Caller ensures the same safety requirements.
        unsafe { self.system.validate_param_unsafe(world) }
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        self.system.initialize(world)
    }

    #[inline]
    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.system.check_change_tick(check);
    }

    #[inline]
    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        self.system.default_system_sets()
    }

    #[inline]
    fn get_last_run(&self) -> Tick {
        self.system.get_last_run()
    }

    #[inline]
    fn set_last_run(&mut self, last_run: Tick) {
        self.system.set_last_run(last_run);
    }
}

/// A [`BoxedCondition`] stored alongside the access returned from [`System::initialize`].
pub struct ConditionWithAccess {
    /// The condition itself.
    pub condition: BoxedCondition,
    /// The access returned by [`System::initialize`].
    /// This will be empty if the system has not been initialized yet.
    pub access: FilteredAccessSet,
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

impl System for ConditionWithAccess {
    type In = ();
    type Out = bool;

    #[inline]
    fn name(&self) -> DebugName {
        self.condition.name()
    }

    #[inline]
    fn type_id(&self) -> TypeId {
        self.condition.type_id()
    }

    #[inline]
    fn flags(&self) -> SystemStateFlags {
        self.condition.flags()
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        // SAFETY: Caller ensures the same safety requirements.
        unsafe { self.condition.run_unsafe(input, world) }
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        self.condition.refresh_hotpatch();
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.condition.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.condition.queue_deferred(world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Caller ensures the same safety requirements.
        unsafe { self.condition.validate_param_unsafe(world) }
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        self.condition.initialize(world)
    }

    #[inline]
    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        self.condition.check_change_tick(check);
    }

    #[inline]
    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        self.condition.default_system_sets()
    }

    #[inline]
    fn get_last_run(&self) -> Tick {
        self.condition.get_last_run()
    }

    #[inline]
    fn set_last_run(&mut self, last_run: Tick) {
        self.condition.set_last_run(last_run);
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

impl GraphNodeId for SystemKey {
    type Adjacent = (SystemKey, Direction);
    type Edge = (SystemKey, SystemKey);

    fn kind(&self) -> &'static str {
        "system"
    }
}

impl GraphNodeId for SystemSetKey {
    type Adjacent = (SystemSetKey, Direction);
    type Edge = (SystemSetKey, SystemSetKey);

    fn kind(&self) -> &'static str {
        "system set"
    }
}

impl TryFrom<NodeId> for SystemKey {
    type Error = SystemSetKey;

    fn try_from(value: NodeId) -> Result<Self, Self::Error> {
        match value {
            NodeId::System(key) => Ok(key),
            NodeId::Set(key) => Err(key),
        }
    }
}

impl TryFrom<NodeId> for SystemSetKey {
    type Error = SystemKey;

    fn try_from(value: NodeId) -> Result<Self, Self::Error> {
        match value {
            NodeId::System(key) => Err(key),
            NodeId::Set(key) => Ok(key),
        }
    }
}

/// Unique identifier for a system or system set stored in a [`ScheduleGraph`].
///
/// [`ScheduleGraph`]: crate::schedule::ScheduleGraph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeId {
    /// Identifier for a system.
    System(SystemKey),
    /// Identifier for a system set.
    Set(SystemSetKey),
}

impl NodeId {
    /// Returns `true` if the identified node is a system.
    pub const fn is_system(&self) -> bool {
        matches!(self, NodeId::System(_))
    }

    /// Returns `true` if the identified node is a system set.
    pub const fn is_set(&self) -> bool {
        matches!(self, NodeId::Set(_))
    }

    /// Returns the system key if the node is a system, otherwise `None`.
    pub const fn as_system(&self) -> Option<SystemKey> {
        match self {
            NodeId::System(system) => Some(*system),
            NodeId::Set(_) => None,
        }
    }

    /// Returns the system set key if the node is a system set, otherwise `None`.
    pub const fn as_set(&self) -> Option<SystemSetKey> {
        match self {
            NodeId::System(_) => None,
            NodeId::Set(set) => Some(*set),
        }
    }
}

impl GraphNodeId for NodeId {
    type Adjacent = CompactNodeIdAndDirection;
    type Edge = CompactNodeIdPair;

    fn kind(&self) -> &'static str {
        match self {
            NodeId::System(n) => n.kind(),
            NodeId::Set(n) => n.kind(),
        }
    }
}

impl From<SystemKey> for NodeId {
    fn from(system: SystemKey) -> Self {
        NodeId::System(system)
    }
}

impl From<SystemSetKey> for NodeId {
    fn from(set: SystemSetKey) -> Self {
        NodeId::Set(set)
    }
}

/// Compact storage of a [`NodeId`] and a [`Direction`].
#[derive(Clone, Copy)]
pub struct CompactNodeIdAndDirection {
    key: KeyData,
    is_system: bool,
    direction: Direction,
}

impl Debug for CompactNodeIdAndDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tuple: (_, _) = (*self).into();
        tuple.fmt(f)
    }
}

impl From<(NodeId, Direction)> for CompactNodeIdAndDirection {
    fn from((id, direction): (NodeId, Direction)) -> Self {
        let key = match id {
            NodeId::System(key) => key.data(),
            NodeId::Set(key) => key.data(),
        };
        let is_system = id.is_system();

        Self {
            key,
            is_system,
            direction,
        }
    }
}

impl From<CompactNodeIdAndDirection> for (NodeId, Direction) {
    fn from(value: CompactNodeIdAndDirection) -> Self {
        let node = match value.is_system {
            true => NodeId::System(value.key.into()),
            false => NodeId::Set(value.key.into()),
        };

        (node, value.direction)
    }
}

/// Compact storage of a [`NodeId`] pair.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct CompactNodeIdPair {
    key_a: KeyData,
    key_b: KeyData,
    is_system_a: bool,
    is_system_b: bool,
}

impl Debug for CompactNodeIdPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tuple: (_, _) = (*self).into();
        tuple.fmt(f)
    }
}

impl From<(NodeId, NodeId)> for CompactNodeIdPair {
    fn from((a, b): (NodeId, NodeId)) -> Self {
        let key_a = match a {
            NodeId::System(index) => index.data(),
            NodeId::Set(index) => index.data(),
        };
        let is_system_a = a.is_system();

        let key_b = match b {
            NodeId::System(index) => index.data(),
            NodeId::Set(index) => index.data(),
        };
        let is_system_b = b.is_system();

        Self {
            key_a,
            key_b,
            is_system_a,
            is_system_b,
        }
    }
}

impl From<CompactNodeIdPair> for (NodeId, NodeId) {
    fn from(value: CompactNodeIdPair) -> Self {
        let a = match value.is_system_a {
            true => NodeId::System(value.key_a.into()),
            false => NodeId::Set(value.key_a.into()),
        };

        let b = match value.is_system_b {
            true => NodeId::System(value.key_b.into()),
            false => NodeId::Set(value.key_b.into()),
        };

        (a, b)
    }
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
        &mut self.nodes[key]
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
