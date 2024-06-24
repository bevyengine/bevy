use bevy_utils::all_tuples;

use crate::{
    schedule::{
        condition::{BoxedCondition, Condition},
        graph_utils::{Ambiguity, Dependency, DependencyKind, GraphInfo},
        set::{InternedSystemSet, IntoSystemSet, SystemSet},
        Chain,
    },
    system::{BoxedSystem, IntoSystem, System},
};

fn new_condition<M>(condition: impl Condition<M>) -> BoxedCondition {
    let condition_system = IntoSystem::into_system(condition);
    assert!(
        condition_system.is_send(),
        "Condition `{}` accesses `NonSend` resources. This is not currently supported.",
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

impl<Marker, F> IntoSystemConfigs<Marker> for F
where
    F: IntoSystem<(), (), Marker>,
{
    fn into_configs(self) -> SystemConfigs {
        SystemConfigs::new_system(Box::new(IntoSystem::into_system(self)))
    }
}

impl IntoSystemConfigs<()> for BoxedSystem<(), ()> {
    fn into_configs(self) -> SystemConfigs {
        SystemConfigs::new_system(self)
    }
}

/// Stores configuration for a single generic node (a system or a system set)
///
/// The configuration includes the node itself, scheduling metadata
/// (hierarchy: in which sets is the node contained,
/// dependencies: before/after which other nodes should this node run)
/// and the run conditions associated with this node.
pub struct NodeConfig<T> {
    pub(crate) node: T,
    /// Hierarchy and dependency metadata for this node
    pub(crate) graph_info: GraphInfo,
    pub(crate) conditions: Vec<BoxedCondition>,
}

/// Stores configuration for a single system.
pub type SystemConfig = NodeConfig<BoxedSystem>;

/// A collections of generic [`NodeConfig`]s.
pub enum NodeConfigs<T> {
    /// Configuration for a single node.
    NodeConfig(NodeConfig<T>),
    /// Configuration for a tuple of nested `Configs` instances.
    Configs {
        /// Configuration for each element of the tuple.
        configs: Vec<NodeConfigs<T>>,
        /// Run conditions applied to everything in the tuple.
        collective_conditions: Vec<BoxedCondition>,
        /// See [`Chain`] for usage.
        chained: Chain,
    },
}

/// A collection of [`SystemConfig`].
pub type SystemConfigs = NodeConfigs<BoxedSystem>;

impl SystemConfigs {
    fn new_system(system: BoxedSystem) -> Self {
        // include system in its default sets
        let sets = system.default_system_sets().into_iter().collect();
        Self::NodeConfig(SystemConfig {
            node: system,
            graph_info: GraphInfo {
                hierarchy: sets,
                ..Default::default()
            },
            conditions: Vec::new(),
        })
    }
}

impl<T> NodeConfigs<T> {
    /// Adds a new boxed system set to the systems.
    pub fn in_set_inner(&mut self, set: InternedSystemSet) {
        match self {
            Self::NodeConfig(config) => {
                config.graph_info.hierarchy.push(set);
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
            Self::NodeConfig(config) => {
                config
                    .graph_info
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
            Self::NodeConfig(config) => {
                config
                    .graph_info
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
            Self::NodeConfig(config) => {
                config
                    .graph_info
                    .dependencies
                    .push(Dependency::new(DependencyKind::BeforeNoSync, set));
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
            Self::NodeConfig(config) => {
                config
                    .graph_info
                    .dependencies
                    .push(Dependency::new(DependencyKind::AfterNoSync, set));
            }
            Self::Configs { configs, .. } => {
                for config in configs {
                    config.after_ignore_deferred_inner(set.intern());
                }
            }
        }
    }

    fn distributive_run_if_inner<M>(&mut self, condition: impl Condition<M> + Clone) {
        match self {
            Self::NodeConfig(config) => {
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
            Self::NodeConfig(config) => {
                ambiguous_with(&mut config.graph_info, set);
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
            Self::NodeConfig(config) => {
                config.graph_info.ambiguous_with = Ambiguity::IgnoreAll;
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
            Self::NodeConfig(config) => {
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
            Self::NodeConfig(_) => { /* no op */ }
            Self::Configs { chained, .. } => {
                *chained = Chain::Yes;
            }
        }
        self
    }

    fn chain_ignore_deferred_inner(mut self) -> Self {
        match &mut self {
            Self::NodeConfig(_) => { /* no op */ }
            Self::Configs { chained, .. } => {
                *chained = Chain::YesIgnoreDeferred;
            }
        }
        self
    }
}

/// Types that can convert into a [`SystemConfigs`].
///
/// This trait is implemented for "systems" (functions whose arguments all implement
/// [`SystemParam`](crate::system::SystemParam)), or tuples thereof.
/// It is a common entry point for system configurations.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::schedule::IntoSystemConfigs;
/// # struct AppMock;
/// # struct Update;
/// # impl AppMock {
/// #     pub fn add_systems<M>(
/// #         &mut self,
/// #         schedule: Update,
/// #         systems: impl IntoSystemConfigs<M>,
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
pub trait IntoSystemConfigs<Marker>
where
    Self: Sized,
{
    /// Convert into a [`SystemConfigs`].
    fn into_configs(self) -> SystemConfigs;

    /// Add these systems to the provided `set`.
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemConfigs {
        self.into_configs().in_set(set)
    }

    /// Runs before all systems in `set`. If `self` has any systems that produce [`Commands`](crate::system::Commands)
    /// or other [`Deferred`](crate::system::Deferred) operations, all systems in `set` will see their effect.
    ///
    /// If automatically inserting [`apply_deferred`](crate::schedule::apply_deferred) like
    /// this isn't desired, use [`before_ignore_deferred`](Self::before_ignore_deferred) instead.
    ///
    /// Note: The given set is not implicitly added to the schedule when this system set is added.
    /// It is safe, but no dependencies will be created.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().before(set)
    }

    /// Run after all systems in `set`. If `set` has any systems that produce [`Commands`](crate::system::Commands)
    /// or other [`Deferred`](crate::system::Deferred) operations, all systems in `self` will see their effect.
    ///
    /// If automatically inserting [`apply_deferred`](crate::schedule::apply_deferred) like
    /// this isn't desired, use [`after_ignore_deferred`](Self::after_ignore_deferred) instead.
    ///
    /// Note: The given set is not implicitly added to the schedule when this system set is added.
    /// It is safe, but no dependencies will be created.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().after(set)
    }

    /// Run before all systems in `set`.
    ///
    /// Unlike [`before`](Self::before), this will not cause the systems in
    /// `set` to wait for the deferred effects of `self` to be applied.
    fn before_ignore_deferred<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().before_ignore_deferred(set)
    }

    /// Run after all systems in `set`.
    ///
    /// Unlike [`after`](Self::after), this will not wait for the deferred
    /// effects of systems in `set` to be applied.
    fn after_ignore_deferred<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().after_ignore_deferred(set)
    }

    /// Add a run condition to each contained system.
    ///
    /// Each system will receive its own clone of the [`Condition`] and will only run
    /// if the `Condition` is true.
    ///
    /// Each individual condition will be evaluated at most once (per schedule run),
    /// right before the corresponding system prepares to run.
    ///
    /// This is equivalent to calling [`run_if`](IntoSystemConfigs::run_if) on each individual
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
    /// Use [`run_if`](IntoSystemSetConfigs::run_if) on a [`SystemSet`] if you want to make sure
    /// that either all or none of the systems are run, or you don't want to evaluate the run
    /// condition for each contained system separately.
    fn distributive_run_if<M>(self, condition: impl Condition<M> + Clone) -> SystemConfigs {
        self.into_configs().distributive_run_if(condition)
    }

    /// Run the systems only if the [`Condition`] is `true`.
    ///
    /// The `Condition` will be evaluated at most once (per schedule run),
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
    /// Use [`distributive_run_if`](IntoSystemConfigs::distributive_run_if) if you want the
    /// condition to be evaluated for each individual system, right before one is run.
    fn run_if<M>(self, condition: impl Condition<M>) -> SystemConfigs {
        self.into_configs().run_if(condition)
    }

    /// Suppress warnings and errors that would result from these systems having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().ambiguous_with(set)
    }

    /// Suppress warnings and errors that would result from these systems having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> SystemConfigs {
        self.into_configs().ambiguous_with_all()
    }

    /// Treat this collection as a sequence of systems.
    ///
    /// Ordering constraints will be applied between the successive elements.
    ///
    /// If the preceding node on a edge has deferred parameters, a [`apply_deferred`](crate::schedule::apply_deferred)
    /// will be inserted on the edge. If this behavior is not desired consider using
    /// [`chain_ignore_deferred`](Self::chain_ignore_deferred) instead.
    fn chain(self) -> SystemConfigs {
        self.into_configs().chain()
    }

    /// Treat this collection as a sequence of systems.
    ///
    /// Ordering constraints will be applied between the successive elements.
    ///
    /// Unlike [`chain`](Self::chain) this will **not** add [`apply_deferred`](crate::schedule::apply_deferred) on the edges.
    fn chain_ignore_deferred(self) -> SystemConfigs {
        self.into_configs().chain_ignore_deferred()
    }
}

impl IntoSystemConfigs<()> for SystemConfigs {
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

    fn distributive_run_if<M>(mut self, condition: impl Condition<M> + Clone) -> SystemConfigs {
        self.distributive_run_if_inner(condition);
        self
    }

    fn run_if<M>(mut self, condition: impl Condition<M>) -> SystemConfigs {
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

#[doc(hidden)]
pub struct SystemConfigTupleMarker;

macro_rules! impl_system_collection {
    ($(($param: ident, $sys: ident)),*) => {
        impl<$($param, $sys),*> IntoSystemConfigs<(SystemConfigTupleMarker, $($param,)*)> for ($($sys,)*)
        where
            $($sys: IntoSystemConfigs<$param>),*
        {
            #[allow(non_snake_case)]
            fn into_configs(self) -> SystemConfigs {
                let ($($sys,)*) = self;
                SystemConfigs::Configs {
                    configs: vec![$($sys.into_configs(),)*],
                    collective_conditions: Vec::new(),
                    chained: Chain::No,
                }
            }
        }
    }
}

all_tuples!(impl_system_collection, 1, 20, P, S);

/// A [`SystemSet`] with scheduling metadata.
pub type SystemSetConfig = NodeConfig<InternedSystemSet>;

impl SystemSetConfig {
    #[track_caller]
    pub(super) fn new(set: InternedSystemSet) -> Self {
        // system type sets are automatically populated
        // to avoid unintentionally broad changes, they cannot be configured
        assert!(
            set.system_type().is_none(),
            "configuring system type sets is not allowed"
        );

        Self {
            node: set,
            graph_info: GraphInfo::default(),
            conditions: Vec::new(),
        }
    }
}

/// A collection of [`SystemSetConfig`].
pub type SystemSetConfigs = NodeConfigs<InternedSystemSet>;

/// Types that can convert into a [`SystemSetConfigs`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not describe a valid system set configuration",
    label = "invalid system set configuration"
)]
pub trait IntoSystemSetConfigs
where
    Self: Sized,
{
    /// Convert into a [`SystemSetConfigs`].
    #[doc(hidden)]
    fn into_configs(self) -> SystemSetConfigs;

    /// Add these system sets to the provided `set`.
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemSetConfigs {
        self.into_configs().in_set(set)
    }

    /// Runs before all systems in `set`. If `self` has any systems that produce [`Commands`](crate::system::Commands)
    /// or other [`Deferred`](crate::system::Deferred) operations, all systems in `set` will see their effect.
    ///
    /// If automatically inserting [`apply_deferred`](crate::schedule::apply_deferred) like
    /// this isn't desired, use [`before_ignore_deferred`](Self::before_ignore_deferred) instead.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().before(set)
    }

    /// Runs before all systems in `set`. If `set` has any systems that produce [`Commands`](crate::system::Commands)
    /// or other [`Deferred`](crate::system::Deferred) operations, all systems in `self` will see their effect.
    ///
    /// If automatically inserting [`apply_deferred`](crate::schedule::apply_deferred) like
    /// this isn't desired, use [`after_ignore_deferred`](Self::after_ignore_deferred) instead.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().after(set)
    }

    /// Run before all systems in `set`.
    ///
    /// Unlike [`before`](Self::before), this will not cause the systems in `set` to wait for the
    /// deferred effects of `self` to be applied.
    fn before_ignore_deferred<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().before_ignore_deferred(set)
    }

    /// Run after all systems in `set`.
    ///
    /// Unlike [`after`](Self::after), this may not see the deferred
    /// effects of systems in `set` to be applied.
    fn after_ignore_deferred<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().after_ignore_deferred(set)
    }

    /// Run the systems in this set(s) only if the [`Condition`] is `true`.
    ///
    /// The `Condition` will be evaluated at most once (per schedule run),
    /// the first time a system in this set(s) prepares to run.
    fn run_if<M>(self, condition: impl Condition<M>) -> SystemSetConfigs {
        self.into_configs().run_if(condition)
    }

    /// Suppress warnings and errors that would result from systems in these sets having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().ambiguous_with(set)
    }

    /// Suppress warnings and errors that would result from systems in these sets having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> SystemSetConfigs {
        self.into_configs().ambiguous_with_all()
    }

    /// Treat this collection as a sequence of system sets.
    ///
    /// Ordering constraints will be applied between the successive elements.
    fn chain(self) -> SystemSetConfigs {
        self.into_configs().chain()
    }

    /// Treat this collection as a sequence of systems.
    ///
    /// Ordering constraints will be applied between the successive elements.
    ///
    /// Unlike [`chain`](Self::chain) this will **not** add [`apply_deferred`](crate::schedule::apply_deferred) on the edges.
    fn chain_ignore_deferred(self) -> SystemSetConfigs {
        self.into_configs().chain_ignore_deferred()
    }
}

impl IntoSystemSetConfigs for SystemSetConfigs {
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

    fn run_if<M>(mut self, condition: impl Condition<M>) -> SystemSetConfigs {
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

impl<S: SystemSet> IntoSystemSetConfigs for S {
    fn into_configs(self) -> SystemSetConfigs {
        SystemSetConfigs::NodeConfig(SystemSetConfig::new(self.intern()))
    }
}

impl IntoSystemSetConfigs for SystemSetConfig {
    fn into_configs(self) -> SystemSetConfigs {
        SystemSetConfigs::NodeConfig(self)
    }
}

macro_rules! impl_system_set_collection {
    ($($set: ident),*) => {
        impl<$($set: IntoSystemSetConfigs),*> IntoSystemSetConfigs for ($($set,)*)
        {
            #[allow(non_snake_case)]
            fn into_configs(self) -> SystemSetConfigs {
                let ($($set,)*) = self;
                SystemSetConfigs::Configs {
                    configs: vec![$($set.into_configs(),)*],
                    collective_conditions: Vec::new(),
                    chained: Chain::No,
                }
            }
        }
    }
}

all_tuples!(impl_system_set_collection, 1, 20, S);
