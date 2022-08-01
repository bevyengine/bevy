use crate::{
    schedule::{SystemLabel, SystemLabelId},
    schedule_v3::{BoxedRunCondition, IntoRunCondition},
    system::{AsSystemLabel, BoxedSystem, IntoSystem, System},
};

use bevy_utils::HashSet;

/// Unique identifier for a system or system set stored in [`Systems`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeId {
    System(u64),
    Set(u64),
}

impl NodeId {
    /// Returns `true` if this identifies a system.
    pub fn is_system(&self) -> bool {
        match self {
            NodeId::System(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if this identifies a system set.
    pub fn is_set(&self) -> bool {
        match self {
            NodeId::Set(_) => true,
            _ => false,
        }
    }

    pub fn type_str(&self) -> &'static str {
        match self {
            NodeId::System(_) => "system",
            NodeId::Set(_) => "system set",
        }
    }
}

/// Pick a consistent ordering for a `NodeId` pair.
pub(crate) fn sort_pair(a: NodeId, b: NodeId) -> (NodeId, NodeId) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Before or after.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub(crate) enum Order {
    Before,
    After,
}

/// Information for dependency graph construction. See [`Systems`](crate::schedule::Systems).
#[derive(Debug, Clone)]
pub(crate) struct NodeInfo {
    pub(crate) name: Option<SystemLabelId>,
    pub(crate) sets: HashSet<SystemLabelId>,
    pub(crate) edges: Vec<(Order, SystemLabelId)>,
}

impl NodeInfo {
    pub(crate) fn name(&self) -> Option<&SystemLabelId> {
        self.name.as_ref()
    }

    pub(crate) fn sets(&self) -> &HashSet<SystemLabelId> {
        &self.sets
    }

    pub(crate) fn edges(&self) -> &[(Order, SystemLabelId)] {
        &self.edges
    }
}

/// Information for system graph construction. See [`Systems`](crate::schedule::Systems).
#[derive(Debug, Clone)]
pub(crate) struct IndexedNodeInfo {
    pub(crate) sets: HashSet<NodeId>,
    pub(crate) edges: Vec<(Order, NodeId)>,
}

impl IndexedNodeInfo {
    pub(crate) fn sets(&self) -> &HashSet<NodeId> {
        &self.sets
    }

    pub(crate) fn edges(&self) -> &[(Order, NodeId)] {
        &self.edges
    }
}

/// Encapsulates a system set and information on when it should run.
pub struct ScheduledSystemSet {
    pub(crate) info: NodeInfo,
    pub(crate) conditions: Vec<BoxedRunCondition>,
}

/// Encapsulates a system and information on when it should run.
pub struct ScheduledSystem {
    pub(crate) system: BoxedSystem,
    pub(crate) info: NodeInfo,
    pub(crate) conditions: Vec<BoxedRunCondition>,
}

fn new_set(label: SystemLabelId) -> ScheduledSystemSet {
    ScheduledSystemSet {
        info: NodeInfo {
            name: Some(label),
            sets: HashSet::new(),
            edges: Vec::new(),
        },
        conditions: Vec::new(),
    }
}

fn new_system(system: BoxedSystem) -> ScheduledSystem {
    ScheduledSystem {
        system,
        info: NodeInfo {
            name: None,
            sets: HashSet::new(),
            edges: Vec::new(),
        },
        conditions: Vec::new(),
    }
}

// TODO: fn prepend_in_set(self, set: impl SystemLabel)
// TODO: fn  append_in_set(self, set: impl SystemLabel)
// TODO: Adds something to a set before / after everything in the set (at time of addition)

/// Types that can be converted into a [`ScheduledSystemSet`].
///
/// Blanket implemented for types the implement [`SystemLabel`] and boxed trait objects.
pub trait IntoScheduledSystemSet: sealed::IntoScheduledSystemSet {
    /// Convert into a `ScheduledSystemSet`.
    fn schedule(self) -> ScheduledSystemSet;
    /// The systems in this set will run under `set`.
    fn in_set(self, set: impl SystemLabel) -> ScheduledSystemSet;
    /// The systems in this set will run before the system or those in the set given by `label`.
    fn before<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystemSet;
    /// The systems in this set will run after the system or those in the set given by `label`.
    fn after<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystemSet;
    /// The systems in this set will run between the systems or those in the sets given by `a` and `b`.
    fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> ScheduledSystemSet;
    /// The systems in this set will run only if `condition` returns `true`.
    fn run_if<P>(self, condition: impl IntoRunCondition<P>) -> ScheduledSystemSet;
}

impl<L> IntoScheduledSystemSet for L
where
    L: SystemLabel + sealed::IntoScheduledSystemSet,
{
    fn schedule(self) -> ScheduledSystemSet {
        new_set(self.as_label())
    }

    fn in_set(self, set: impl SystemLabel) -> ScheduledSystemSet {
        new_set(self.as_label()).in_set(set)
    }

    fn before<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystemSet {
        new_set(self.as_label()).before(label)
    }

    fn after<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystemSet {
        new_set(self.as_label()).after(label)
    }

    fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> ScheduledSystemSet {
        new_set(self.as_label()).after(a).before(b)
    }

    fn run_if<P>(self, condition: impl IntoRunCondition<P>) -> ScheduledSystemSet {
        new_set(self.as_label()).run_if(condition)
    }
}

impl IntoScheduledSystemSet for ScheduledSystemSet {
    fn schedule(self) -> ScheduledSystemSet {
        self
    }

    fn in_set(mut self, set: impl SystemLabel) -> ScheduledSystemSet {
        self.info.sets.insert(set.as_label());
        self
    }

    fn before<M>(mut self, label: impl AsSystemLabel<M>) -> ScheduledSystemSet {
        self.info
            .edges
            .push((Order::Before, label.as_system_label().as_label()));
        self
    }

    fn after<M>(mut self, label: impl AsSystemLabel<M>) -> ScheduledSystemSet {
        self.info
            .edges
            .push((Order::After, label.as_system_label().as_label()));
        self
    }

    fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> ScheduledSystemSet {
        self.after(a).before(b)
    }

    fn run_if<P>(mut self, condition: impl IntoRunCondition<P>) -> ScheduledSystemSet {
        self.conditions
            .push(Box::new(IntoSystem::into_system(condition)));
        self
    }
}

/// Types that can be converted into a [`ScheduledSystem`].
///
/// Blanked implemented for types that become [`System<In=(), Out=()>`](crate::system::System)
/// and boxed trait objects.
pub trait IntoScheduledSystem<Params>: sealed::IntoScheduledSystem<Params> {
    /// Convert into a `ScheduledSystem`.
    fn schedule(self) -> ScheduledSystem;
    /// Sets `name` as the unique label for this instance of the system.
    fn named(self, name: impl SystemLabel) -> ScheduledSystem;
    /// This system will run under the set given by `label`.
    fn in_set(self, set: impl SystemLabel) -> ScheduledSystem;
    /// This system will run before the system or those in the set given by `label`.
    fn before<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystem;
    /// This system will run after the system or those in the set given by `label`.
    fn after<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystem;
    /// This system will run between the systems or those in the sets given by `a` and `b`.
    fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> ScheduledSystem;
    /// This system will run only if `condition` returns `true`.
    fn run_if<P>(self, condition: impl IntoRunCondition<P>) -> ScheduledSystem;
}

impl<Params, F> IntoScheduledSystem<Params> for F
where
    F: IntoSystem<(), (), Params> + sealed::IntoScheduledSystem<Params>,
{
    fn schedule(self) -> ScheduledSystem {
        new_system(Box::new(IntoSystem::into_system(self)))
    }

    fn named(self, name: impl SystemLabel) -> ScheduledSystem {
        new_system(Box::new(IntoSystem::into_system(self))).named(name)
    }

    fn in_set(self, set: impl SystemLabel) -> ScheduledSystem {
        new_system(Box::new(IntoSystem::into_system(self))).in_set(set)
    }

    fn before<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystem {
        new_system(Box::new(IntoSystem::into_system(self))).before(label)
    }

    fn after<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystem {
        new_system(Box::new(IntoSystem::into_system(self))).after(label)
    }

    fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> ScheduledSystem {
        new_system(Box::new(IntoSystem::into_system(self)))
            .after(a)
            .before(b)
    }

    fn run_if<P>(self, condition: impl IntoRunCondition<P>) -> ScheduledSystem {
        new_system(Box::new(IntoSystem::into_system(self))).run_if(condition)
    }
}

impl IntoScheduledSystem<()> for BoxedSystem<(), ()> {
    fn schedule(self) -> ScheduledSystem {
        new_system(self)
    }

    fn named(self, name: impl SystemLabel) -> ScheduledSystem {
        new_system(self).named(name)
    }

    fn in_set(self, set: impl SystemLabel) -> ScheduledSystem {
        new_system(self).in_set(set)
    }

    fn before<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystem {
        new_system(self).before(label)
    }

    fn after<M>(self, label: impl AsSystemLabel<M>) -> ScheduledSystem {
        new_system(self).after(label)
    }

    fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> ScheduledSystem {
        new_system(self).after(a).before(b)
    }

    fn run_if<P>(self, condition: impl IntoRunCondition<P>) -> ScheduledSystem {
        new_system(self).run_if(condition)
    }
}

impl IntoScheduledSystem<()> for ScheduledSystem {
    fn schedule(self) -> ScheduledSystem {
        self
    }

    fn named(mut self, name: impl SystemLabel) -> ScheduledSystem {
        self.info.name = Some(name.as_label());
        self
    }

    fn in_set(mut self, set: impl SystemLabel) -> ScheduledSystem {
        self.info.sets.insert(set.as_label());
        self
    }

    fn before<M>(mut self, label: impl AsSystemLabel<M>) -> ScheduledSystem {
        self.info
            .edges
            .push((Order::Before, label.as_system_label().as_label()));
        self
    }

    fn after<M>(mut self, label: impl AsSystemLabel<M>) -> ScheduledSystem {
        self.info
            .edges
            .push((Order::After, label.as_system_label().as_label()));
        self
    }

    fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> ScheduledSystem {
        self.after(a).before(b)
    }

    fn run_if<P>(mut self, condition: impl IntoRunCondition<P>) -> ScheduledSystem {
        self.conditions
            .push(Box::new(IntoSystem::into_system(condition)));
        self
    }
}

mod sealed {
    use crate::{
        schedule::SystemLabel,
        system::{BoxedSystem, IntoSystem},
    };

    use super::{ScheduledSystem, ScheduledSystemSet};

    // These traits are private because non-`()` systems cannot be used.
    // The type system doesn't allow for mixed type collections.
    // Maybe we could do funky transmutes on the fn pointers like we do for `CommandQueue`.
    pub trait IntoScheduledSystem<Params> {}

    impl<Params, F: IntoSystem<(), (), Params>> IntoScheduledSystem<Params> for F {}

    impl IntoScheduledSystem<()> for BoxedSystem<(), ()> {}

    impl IntoScheduledSystem<()> for ScheduledSystem {}

    pub trait IntoScheduledSystemSet {}

    impl<L: SystemLabel> IntoScheduledSystemSet for L {}

    impl IntoScheduledSystemSet for ScheduledSystemSet {}
}

pub use bulk::*;

/// Provides types and macros to allow info multiple things in bulk.
mod bulk {
    use super::*;

    /// Common wrapper type for system and system set descriptors.
    pub enum Scheduled {
        System(ScheduledSystem),
        Set(ScheduledSystemSet),
    }

    impl Scheduled {
        fn name(&self) -> SystemLabelId {
            match self {
                Self::System(system) => system.info.name().unwrap().as_label(),
                Self::Set(set) => set.info.name().unwrap().as_label(),
            }
        }
    }

    impl ScheduledSystemSet {
        pub fn into_common(self) -> Scheduled {
            Scheduled::Set(self)
        }
    }

    impl ScheduledSystem {
        pub fn into_common(self) -> Scheduled {
            Scheduled::System(self)
        }
    }

    #[doc(hidden)]
    /// Describes a group of systems and system sets, in no particular order.
    pub struct Group(Vec<Scheduled>);

    impl Group {
        pub fn new(mut vec: Vec<Scheduled>) -> Self {
            Self(vec)
        }

        /// Configures the nodes to run below the set given by `label`.
        pub fn in_set(mut self, label: impl SystemLabel) -> Self {
            for node in self.0.iter_mut() {
                match node {
                    Scheduled::System(system) => {
                        system.info.sets.insert(label.as_label());
                    }
                    Scheduled::Set(set) => {
                        set.info.sets.insert(label.as_label());
                    }
                };
            }

            self
        }

        /// Configures the nodes to run before the system or set given by `label`.
        pub fn before<M>(mut self, label: impl AsSystemLabel<M>) -> Self {
            for node in self.0.iter_mut() {
                match node {
                    Scheduled::System(system) => {
                        system
                            .info
                            .edges
                            .push((Order::Before, label.as_system_label().as_label()));
                    }
                    Scheduled::Set(set) => {
                        set.info
                            .edges
                            .push((Order::Before, label.as_system_label().as_label()));
                    }
                }
            }

            self
        }

        /// Configures the nodes to run after the system or set given by `label`.
        pub fn after<M>(mut self, label: impl AsSystemLabel<M>) -> Self {
            for node in self.0.iter_mut() {
                match node {
                    Scheduled::System(system) => {
                        system
                            .info
                            .edges
                            .push((Order::After, label.as_system_label().as_label()));
                    }
                    Scheduled::Set(set) => {
                        set.info
                            .edges
                            .push((Order::After, label.as_system_label().as_label()));
                    }
                }
            }

            self
        }

        /// The system will run between the systems or sets given by `a` and `b`.
        pub fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> Self {
            self.after(a).before(b)
        }
    }

    impl IntoIterator for Group {
        type Item = Scheduled;
        type IntoIter = std::vec::IntoIter<Scheduled>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    #[doc(hidden)]
    /// Describes a group of systems and system sets that are ordered in a sequence.
    pub struct Chain(Vec<Scheduled>);

    impl Chain {
        pub fn new(mut vec: Vec<Scheduled>) -> Self {
            let n = vec.len();
            let names = vec.iter().skip(1).map(|c| c.name()).collect::<Vec<_>>();
            for (node, next_node) in vec.iter_mut().take(n - 1).zip(names.into_iter()) {
                match node {
                    Scheduled::System(system) => {
                        system.info.edges.push((Order::Before, next_node));
                    }
                    Scheduled::Set(set) => {
                        set.info.edges.push((Order::Before, next_node));
                    }
                }
            }

            Self(vec)
        }

        /// Configures the nodes to run below the set given by `label`.
        pub fn in_set(mut self, label: impl SystemLabel) -> Self {
            for node in self.0.iter_mut() {
                match node {
                    Scheduled::System(system) => {
                        system.info.sets.insert(label.as_label());
                    }
                    Scheduled::Set(set) => {
                        set.info.sets.insert(label.as_label());
                    }
                };
            }

            self
        }

        /// Configures the nodes to run before the system or set given by `label`.
        pub fn before<M>(mut self, label: impl AsSystemLabel<M>) -> Self {
            if let Some(last) = self.0.last_mut() {
                match last {
                    Scheduled::System(system) => {
                        system
                            .info
                            .edges
                            .push((Order::Before, label.as_system_label().as_label()));
                    }
                    Scheduled::Set(set) => {
                        set.info
                            .edges
                            .push((Order::Before, label.as_system_label().as_label()));
                    }
                }
            }

            self
        }

        /// Configures the nodes to run after the system or set given by `label`.
        pub fn after<M>(mut self, label: impl AsSystemLabel<M>) -> Self {
            if let Some(first) = self.0.first_mut() {
                match first {
                    Scheduled::System(system) => {
                        system
                            .info
                            .edges
                            .push((Order::After, label.as_system_label().as_label()));
                    }
                    Scheduled::Set(set) => {
                        set.info
                            .edges
                            .push((Order::After, label.as_system_label().as_label()));
                    }
                }
            }

            self
        }

        /// Configures the nodes to run between the systems or sets given by `a` and `b`.
        pub fn between<M>(self, a: impl AsSystemLabel<M>, b: impl AsSystemLabel<M>) -> Self {
            self.after(a).before(b)
        }
    }

    impl IntoIterator for Chain {
        type Item = Scheduled;
        type IntoIter = std::vec::IntoIter<Scheduled>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    /// A mixed group of systems and system sets with no particular order.
    #[macro_export]
    macro_rules! par {
        ($($x:expr),+ $(,)?) => {
            bevy_ecs::schedule::Group::new(vec![$(($x).schedule().into_common()),+])
        };
    }

    pub use par;

    /// A mixed group of systems and system sets with a sequential order.
    #[macro_export]
    macro_rules! seq {
        ($($x:expr),+ $(,)?) => {
            bevy_ecs::schedule::Chain::new(vec![$(($x).schedule().into_common()),+])
        };
    }

    pub use seq;
}
