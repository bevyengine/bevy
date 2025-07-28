use alloc::{boxed::Box, vec, vec::Vec};
use variadics_please::all_tuples;

use crate::{
    schedule::{
        auto_insert_apply_deferred::IgnoreDeferred,
        condition::{BoxedCondition, SystemCondition},
        graph::{Ambiguity, Dependency, DependencyKind, GraphInfo},
        set::{InternedSystemSet, IntoSystemSet, SystemSet},
        Chain,
    },
    system::{BoxedSystem, IntoSystem, ScheduleSystem, System},
};

fn new_condition<M>(condition: impl SystemCondition<M>) -> BoxedCondition {
    let condition_system = IntoSystem::into_system(condition);
    assert!(
        condition_system.is_send(),
        "SystemCondition `{}` accesses `NonSend` resources. This is not currently supported.",
        condition_system.name()
    );

    Box::new(condition_system)
}

fn ambiguous_with(graph_info: &mut GraphInfo, set: InternedSystemSet) {
    match &mut graph_info.ambiguous_with {
        detection @ Ambiguity::Check => {
            *detection = Ambiguity::IgnoreWithSet(vec![set]);
        }
        Ambiguity::IgnoreWithSet(ambiguous_with) => {
            ambiguous_with.push(set);
        }
        Ambiguity::IgnoreAll => (),
    }
}

/// Stores data to differentiate different schedulable structs.
pub trait Schedulable {
    /// Additional data used to configure independent scheduling. Stored in [`ScheduleConfig`].
    type Metadata;
    /// Additional data used to configure a schedulable group. Stored in [`ScheduleConfigs`].
    type GroupMetadata;

    /// Initializes a configuration from this node.
    fn into_config(self) -> ScheduleConfig<Self>
    where
        Self: Sized;
}

impl Schedulable for ScheduleSystem {
    type Metadata = GraphInfo;
    type GroupMetadata = Chain;

    fn into_config(self) -> ScheduleConfig<Self> {
        let sets = self.default_system_sets().clone();
        ScheduleConfig {
            node: self,
            metadata: GraphInfo {
                hierarchy: sets,
                ..Default::default()
            },
            conditions: Vec::new(),
        }
    }
}

impl Schedulable for InternedSystemSet {
    type Metadata = GraphInfo;
    type GroupMetadata = Chain;

    fn into_config(self) -> ScheduleConfig<Self> {
        assert!(
            self.system_type().is_none(),
            "configuring system type sets is not allowed"
        );

        ScheduleConfig {
            node: self,
            metadata: GraphInfo::default(),
            conditions: Vec::new(),
        }
    }
}

/// Stores configuration for a single generic node (a system or a system set)
///
/// The configuration includes the node itself, scheduling metadata
/// (hierarchy: in which sets is the node contained,
/// dependencies: before/after which other nodes should this node run)
/// and the run conditions associated with this node.
pub struct ScheduleConfig<T: Schedulable> {
    pub(crate) node: T,
    pub(crate) metadata: T::Metadata,
    pub(crate) conditions: Vec<BoxedCondition>,
}

/// Single or nested configurations for [`Schedulable`]s.
pub enum ScheduleConfigs<T: Schedulable> {
    /// Configuration for a single [`Schedulable`].
    ScheduleConfig(ScheduleConfig<T>),
    /// Configuration for a tuple of nested `Configs` instances.
    Configs {
        /// Configuration for each element of the tuple.
        configs: Vec<ScheduleConfigs<T>>,
        /// Run conditions applied to everything in the tuple.
        collective_conditions: Vec<BoxedCondition>,
        /// Metadata to be applied to all elements in the tuple.
        metadata: T::GroupMetadata,
    },
}

impl<T: Schedulable<Metadata = GraphInfo, GroupMetadata = Chain>> ScheduleConfigs<T> {
    /// Adds a new boxed system set to the systems.
    pub fn in_set_inner(&mut self, set: InternedSystemSet) {
        match self {
            Self::ScheduleConfig(config) => {
                config.metadata.hierarchy.push(set);
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.in_set_inner(set);
                }
            }
        }
    }

    fn before_inner(&mut self, set: InternedSystemSet) {
        match self {
            Self::ScheduleConfig(config) => {
                config
                    .metadata
                    .dependencies
                    .push(Dependency::new(DependencyKind::Before, set));
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.before_inner(set);
                }
            }
        }
    }

    fn after_inner(&mut self, set: InternedSystemSet) {
        match self {
            Self::ScheduleConfig(config) => {
                config
                    .metadata
                    .dependencies
                    .push(Dependency::new(DependencyKind::After, set));
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.after_inner(set);
                }
            }
        }
    }

    fn before_ignore_deferred_inner(&mut self, set: InternedSystemSet) {
        match self {
            Self::ScheduleConfig(config) => {
                config
                    .metadata
                    .dependencies
                    .push(Dependency::new(DependencyKind::Before, set).add_config(IgnoreDeferred));
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.before_ignore_deferred_inner(set.intern());
                }
            }
        }
    }

    fn after_ignore_deferred_inner(&mut self, set: InternedSystemSet) {
        match self {
            Self::ScheduleConfig(config) => {
                config
                    .metadata
                    .dependencies
                    .push(Dependency::new(DependencyKind::After, set).add_config(IgnoreDeferred));
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.after_ignore_deferred_inner(set.intern());
                }
            }
        }
    }

    fn distributive_run_if_inner<M>(&mut self, condition: impl SystemCondition<M> + Clone) {
        match self {
            Self::ScheduleConfig(config) => {
                config.conditions.push(new_condition(condition));
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.distributive_run_if_inner(condition.clone());
                }
            }
        }
    }

    fn ambiguous_with_inner(&mut self, set: InternedSystemSet) {
        match self {
            Self::ScheduleConfig(config) => {
                ambiguous_with(&mut config.metadata, set);
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.ambiguous_with_inner(set);
                }
            }
        }
    }

    fn ambiguous_with_all_inner(&mut self) {
        match self {
            Self::ScheduleConfig(config) => {
                config.metadata.ambiguous_with = Ambiguity::IgnoreAll;
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.ambiguous_with_all_inner();
                }
            }
        }
    }

    /// Adds a new boxed run condition to the systems.
    ///
    /// This is useful if you have a run condition whose concrete type is unknown.
    /// Prefer `run_if` for run conditions whose type is known at compile time.
    pub fn run_if_dyn(&mut self, condition: BoxedCondition) {
        match self {
            Self::ScheduleConfig(config) => {
                config.conditions.push(condition);
            }
            Self::Configs {
                collective_conditions,
                ..
            } => {
                collective_conditions.push(condition);
            }
        }
    }

    fn chain_inner(mut self) -> Self {
        match &mut self {
            Self::ScheduleConfig(_) => { /* no op */ }
            Self::Configs { metadata, .. } => {
                metadata.set_chained();
            }
        };
        self
    }

    fn chain_ignore_deferred_inner(mut self) -> Self {
        match &mut self {
            Self::ScheduleConfig(_) => { /* no op */ }
            Self::Configs { metadata, .. } => {
                metadata.set_chained_with_config(IgnoreDeferred);
            }
        }
        self
    }
}

/// Types that can convert into a [`ScheduleConfigs`].
///
/// This trait is implemented for "systems" (functions whose arguments all implement
/// [`SystemParam`](crate::system::SystemParam)), or tuples thereof.
/// It is a common entry point for system configurations.
///
/// # Usage notes
///
/// This trait should only be used as a bound for trait implementations or as an
/// argument to a function. If system configs need to be returned from a
/// function or stored somewhere, use [`ScheduleConfigs`] instead of this trait.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::{schedule::IntoScheduleConfigs, system::ScheduleSystem};
/// # struct AppMock;
/// # struct Update;
/// # impl AppMock {
/// #     pub fn add_systems<M>(
/// #         &mut self,
/// #         schedule: Update,
/// #         systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
/// #    ) -> &mut Self { self }
/// # }
/// # let mut app = AppMock;
///
/// fn handle_input() {}
///
/// fn update_camera() {}
/// fn update_character() {}
///
/// app.add_systems(
///     Update,
///     (
///         handle_input,
///         (update_camera, update_character).after(handle_input)
///     )
/// );
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not describe a valid system configuration",
    label = "invalid system configuration"
)]
pub trait IntoScheduleConfigs<T: Schedulable<Metadata = GraphInfo, GroupMetadata = Chain>, Marker>:
    Sized
{
    /// Convert into a [`ScheduleConfigs`].
    fn into_configs(self) -> ScheduleConfigs<T>;

    /// Add these systems to the provided `set`.
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> ScheduleConfigs<T> {
        self.into_configs().in_set(set)
    }

    /// Runs before all systems in `set`. If `self` has any systems that produce [`Commands`](crate::system::Commands)
    /// or other [`Deferred`](crate::system::Deferred) operations, all systems in `set` will see their effect.
    ///
    /// If automatically inserting [`ApplyDeferred`](crate::schedule::ApplyDeferred) like
    /// this isn't desired, use [`before_ignore_deferred`](Self::before_ignore_deferred) instead.
    ///
    /// Calling [`.chain`](Self::chain) is often more convenient and ensures that all systems are added to the schedule.
    /// Please check the [caveats section of `.after`](Self::after) for details.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> ScheduleConfigs<T> {
        self.into_configs().before(set)
    }

    /// Run after all systems in `set`. If `set` has any systems that produce [`Commands`](crate::system::Commands)
    /// or other [`Deferred`](crate::system::Deferred) operations, all systems in `self` will see their effect.
    ///
    /// If automatically inserting [`ApplyDeferred`](crate::schedule::ApplyDeferred) like
    /// this isn't desired, use [`after_ignore_deferred`](Self::after_ignore_deferred) instead.
    ///
    /// Calling [`.chain`](Self::chain) is often more convenient and ensures that all systems are added to the schedule.
    ///
    /// # Caveats
    ///
    /// If you configure two [`System`]s like `(GameSystem::A).after(GameSystem::B)` or `(GameSystem::A).before(GameSystem::B)`, the `GameSystem::B` will not be automatically scheduled.
    ///
    /// This means that the system `GameSystem::A` and the system or systems in `GameSystem::B` will run independently of each other if `GameSystem::B` was never explicitly scheduled with [`configure_sets`]
    /// If that is the case, `.after`/`.before` will not provide the desired behavior
    /// and the systems can run in parallel or in any order determined by the scheduler.
    /// Only use `after(GameSystem::B)` and `before(GameSystem::B)` when you know that `B` has already been scheduled for you,
    /// e.g. when it was provided by Bevy or a third-party dependency,
    /// or you manually scheduled it somewhere else in your app.
    ///
    /// Another caveat is that if `GameSystem::B` is placed in a different schedule than `GameSystem::A`,
    /// any ordering calls between them—whether using `.before`, `.after`, or `.chain`—will be silently ignored.
    ///
    /// [`configure_sets`]: https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.configure_sets
    fn after<M>(self, set: impl IntoSystemSet<M>) -> ScheduleConfigs<T> {
        self.into_configs().after(set)
    }

    /// Run before all systems in `set`.
    ///
    /// Unlike [`before`](Self::before), this will not cause the systems in
    /// `set` to wait for the deferred effects of `self` to be applied.
    fn before_ignore_deferred<M>(self, set: impl IntoSystemSet<M>) -> ScheduleConfigs<T> {
        self.into_configs().before_ignore_deferred(set)
    }

    /// Run after all systems in `set`.
    ///
    /// Unlike [`after`](Self::after), this will not wait for the deferred
    /// effects of systems in `set` to be applied.
    fn after_ignore_deferred<M>(self, set: impl IntoSystemSet<M>) -> ScheduleConfigs<T> {
        self.into_configs().after_ignore_deferred(set)
    }

    /// Add a run condition to each contained system.
    ///
    /// Each system will receive its own clone of the [`SystemCondition`] and will only run
    /// if the `SystemCondition` is true.
    ///
    /// Each individual condition will be evaluated at most once (per schedule run),
    /// right before the corresponding system prepares to run.
    ///
    /// This is equivalent to calling [`run_if`](IntoScheduleConfigs::run_if) on each individual
    /// system, as shown below:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut schedule = Schedule::default();
    /// # fn a() {}
    /// # fn b() {}
    /// # fn condition() -> bool { true }
    /// schedule.add_systems((a, b).distributive_run_if(condition));
    /// schedule.add_systems((a.run_if(condition), b.run_if(condition)));
    /// ```
    ///
    /// # Note
    ///
    /// Because the conditions are evaluated separately for each system, there is no guarantee
    /// that all evaluations in a single schedule run will yield the same result. If another
    /// system is run inbetween two evaluations it could cause the result of the condition to change.
    ///
    /// Use [`run_if`](ScheduleConfigs::run_if) on a [`SystemSet`] if you want to make sure
    /// that either all or none of the systems are run, or you don't want to evaluate the run
    /// condition for each contained system separately.
    fn distributive_run_if<M>(
        self,
        condition: impl SystemCondition<M> + Clone,
    ) -> ScheduleConfigs<T> {
        self.into_configs().distributive_run_if(condition)
    }

    /// Run the systems only if the [`SystemCondition`] is `true`.
    ///
    /// The `SystemCondition` will be evaluated at most once (per schedule run),
    /// the first time a system in this set prepares to run.
    ///
    /// If this set contains more than one system, calling `run_if` is equivalent to adding each
    /// system to a common set and configuring the run condition on that set, as shown below:
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut schedule = Schedule::default();
    /// # fn a() {}
    /// # fn b() {}
    /// # fn condition() -> bool { true }
    /// # #[derive(SystemSet, Debug, Eq, PartialEq, Hash, Clone, Copy)]
    /// # struct C;
    /// schedule.add_systems((a, b).run_if(condition));
    /// schedule.add_systems((a, b).in_set(C)).configure_sets(C.run_if(condition));
    /// ```
    ///
    /// # Note
    ///
    /// Because the condition will only be evaluated once, there is no guarantee that the condition
    /// is upheld after the first system has run. You need to make sure that no other systems that
    /// could invalidate the condition are scheduled inbetween the first and last run system.
    ///
    /// Use [`distributive_run_if`](IntoScheduleConfigs::distributive_run_if) if you want the
    /// condition to be evaluated for each individual system, right before one is run.
    fn run_if<M>(self, condition: impl SystemCondition<M>) -> ScheduleConfigs<T> {
        self.into_configs().run_if(condition)
    }

    /// Suppress warnings and errors that would result from these systems having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> ScheduleConfigs<T> {
        self.into_configs().ambiguous_with(set)
    }

    /// Suppress warnings and errors that would result from these systems having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> ScheduleConfigs<T> {
        self.into_configs().ambiguous_with_all()
    }

    /// Treat this collection as a sequence of systems.
    ///
    /// Ordering constraints will be applied between the successive elements.
    ///
    /// If the preceding node on an edge has deferred parameters, an [`ApplyDeferred`](crate::schedule::ApplyDeferred)
    /// will be inserted on the edge. If this behavior is not desired consider using
    /// [`chain_ignore_deferred`](Self::chain_ignore_deferred) instead.
    fn chain(self) -> ScheduleConfigs<T> {
        self.into_configs().chain()
    }

    /// Treat this collection as a sequence of systems.
    ///
    /// Ordering constraints will be applied between the successive elements.
    ///
    /// Unlike [`chain`](Self::chain) this will **not** add [`ApplyDeferred`](crate::schedule::ApplyDeferred) on the edges.
    fn chain_ignore_deferred(self) -> ScheduleConfigs<T> {
        self.into_configs().chain_ignore_deferred()
    }
}

impl<T: Schedulable<Metadata = GraphInfo, GroupMetadata = Chain>> IntoScheduleConfigs<T, ()>
    for ScheduleConfigs<T>
{
    fn into_configs(self) -> Self {
        self
    }

    #[track_caller]
    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            set.system_type().is_none(),
            "adding arbitrary systems to a system type set is not allowed"
        );

        self.in_set_inner(set.intern());

        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.before_inner(set.intern());
        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.after_inner(set.intern());
        self
    }

    fn before_ignore_deferred<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.before_ignore_deferred_inner(set.intern());
        self
    }

    fn after_ignore_deferred<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.after_ignore_deferred_inner(set.intern());
        self
    }

    fn distributive_run_if<M>(
        mut self,
        condition: impl SystemCondition<M> + Clone,
    ) -> ScheduleConfigs<T> {
        self.distributive_run_if_inner(condition);
        self
    }

    fn run_if<M>(mut self, condition: impl SystemCondition<M>) -> ScheduleConfigs<T> {
        self.run_if_dyn(new_condition(condition));
        self
    }

    fn ambiguous_with<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.ambiguous_with_inner(set.intern());
        self
    }

    fn ambiguous_with_all(mut self) -> Self {
        self.ambiguous_with_all_inner();
        self
    }

    fn chain(self) -> Self {
        self.chain_inner()
    }

    fn chain_ignore_deferred(self) -> Self {
        self.chain_ignore_deferred_inner()
    }
}

impl<F, Marker> IntoScheduleConfigs<ScheduleSystem, Marker> for F
where
    F: IntoSystem<(), (), Marker>,
{
    fn into_configs(self) -> ScheduleConfigs<ScheduleSystem> {
        let boxed_system = Box::new(IntoSystem::into_system(self));
        ScheduleConfigs::ScheduleConfig(ScheduleSystem::into_config(boxed_system))
    }
}

impl IntoScheduleConfigs<ScheduleSystem, ()> for BoxedSystem<(), ()> {
    fn into_configs(self) -> ScheduleConfigs<ScheduleSystem> {
        ScheduleConfigs::ScheduleConfig(ScheduleSystem::into_config(self))
    }
}

impl<S: SystemSet> IntoScheduleConfigs<InternedSystemSet, ()> for S {
    fn into_configs(self) -> ScheduleConfigs<InternedSystemSet> {
        ScheduleConfigs::ScheduleConfig(InternedSystemSet::into_config(self.intern()))
    }
}

#[doc(hidden)]
pub struct ScheduleConfigTupleMarker;

macro_rules! impl_node_type_collection {
    ($(#[$meta:meta])* $(($param: ident, $sys: ident)),*) => {
        $(#[$meta])*
        impl<$($param, $sys),*, T: Schedulable<Metadata = GraphInfo, GroupMetadata = Chain>> IntoScheduleConfigs<T, (ScheduleConfigTupleMarker, $($param,)*)> for ($($sys,)*)
        where
            $($sys: IntoScheduleConfigs<T, $param>),*
        {
            #[expect(
                clippy::allow_attributes,
                reason = "We are inside a macro, and as such, `non_snake_case` is not guaranteed to apply."
            )]
            #[allow(
                non_snake_case,
                reason = "Variable names are provided by the macro caller, not by us."
            )]
            fn into_configs(self) -> ScheduleConfigs<T> {
                let ($($sys,)*) = self;
                ScheduleConfigs::Configs {
                    metadata: Default::default(),
                    configs: vec![$($sys.into_configs(),)*],
                    collective_conditions: Vec::new(),
                }
            }
        }
    }
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_node_type_collection,
    1,
    20,
    P,
    S
);
