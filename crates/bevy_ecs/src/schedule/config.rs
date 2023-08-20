use bevy_utils::all_tuples;

use crate::{
    schedule::{
        condition::{BoxedCondition, Condition},
        graph_utils::{Ambiguity, Dependency, DependencyKind, GraphInfo},
        set::{BoxedSystemSet, IntoSystemSet, SystemSet},
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

fn ambiguous_with(graph_info: &mut GraphInfo, set: BoxedSystemSet) {
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

/// Stores configuration for a single system.
pub struct SystemConfig {
    pub(crate) system: BoxedSystem,
    pub(crate) graph_info: GraphInfo,
    pub(crate) conditions: Vec<BoxedCondition>,
}

/// A collection of [`SystemConfig`].
pub enum SystemConfigs {
    /// Configuration for a single system.
    SystemConfig(SystemConfig),
    /// Configuration for a tuple of nested `SystemConfigs` instances.
    Configs {
        /// Configuration for each element of the tuple.
        configs: Vec<SystemConfigs>,
        /// Run conditions applied to everything in the tuple.
        collective_conditions: Vec<BoxedCondition>,
        /// If `true`, adds `before -> after` ordering constraints between the successive elements.
        chained: bool,
    },
}

impl SystemConfigs {
    fn new_system(system: BoxedSystem) -> Self {
        // include system in its default sets
        let sets = system.default_system_sets().into_iter().collect();
        Self::SystemConfig(SystemConfig {
            system,
            graph_info: GraphInfo {
                sets,
                ..Default::default()
            },
            conditions: Vec::new(),
        })
    }

    pub(crate) fn in_set_inner(&mut self, set: BoxedSystemSet) {
        match self {
            SystemConfigs::SystemConfig(config) => {
                config.graph_info.sets.push(set);
            }
            SystemConfigs::Configs { configs, .. } => {
                for config in configs {
                    config.in_set_inner(set.dyn_clone());
                }
            }
        }
    }

    fn before_inner(&mut self, set: BoxedSystemSet) {
        match self {
            SystemConfigs::SystemConfig(config) => {
                config
                    .graph_info
                    .dependencies
                    .push(Dependency::new(DependencyKind::Before, set));
            }
            SystemConfigs::Configs { configs, .. } => {
                for config in configs {
                    config.before_inner(set.dyn_clone());
                }
            }
        }
    }

    fn after_inner(&mut self, set: BoxedSystemSet) {
        match self {
            SystemConfigs::SystemConfig(config) => {
                config
                    .graph_info
                    .dependencies
                    .push(Dependency::new(DependencyKind::After, set));
            }
            SystemConfigs::Configs { configs, .. } => {
                for config in configs {
                    config.after_inner(set.dyn_clone());
                }
            }
        }
    }

    fn distributive_run_if_inner<M>(&mut self, condition: impl Condition<M> + Clone) {
        match self {
            SystemConfigs::SystemConfig(config) => {
                config.conditions.push(new_condition(condition));
            }
            SystemConfigs::Configs { configs, .. } => {
                for config in configs {
                    config.distributive_run_if_inner(condition.clone());
                }
            }
        }
    }

    fn ambiguous_with_inner(&mut self, set: BoxedSystemSet) {
        match self {
            SystemConfigs::SystemConfig(config) => {
                ambiguous_with(&mut config.graph_info, set);
            }
            SystemConfigs::Configs { configs, .. } => {
                for config in configs {
                    config.ambiguous_with_inner(set.dyn_clone());
                }
            }
        }
    }

    fn ambiguous_with_all_inner(&mut self) {
        match self {
            SystemConfigs::SystemConfig(config) => {
                config.graph_info.ambiguous_with = Ambiguity::IgnoreAll;
            }
            SystemConfigs::Configs { configs, .. } => {
                for config in configs {
                    config.ambiguous_with_all_inner();
                }
            }
        }
    }

    pub(crate) fn run_if_inner(&mut self, condition: BoxedCondition) {
        match self {
            SystemConfigs::SystemConfig(config) => {
                config.conditions.push(condition);
            }
            SystemConfigs::Configs {
                collective_conditions,
                ..
            } => {
                collective_conditions.push(condition);
            }
        }
    }
}

/// Types that can convert into a [`SystemConfigs`].
pub trait IntoSystemConfigs<Marker>
where
    Self: Sized,
{
    /// Convert into a [`SystemConfigs`].
    #[doc(hidden)]
    fn into_configs(self) -> SystemConfigs;

    /// Add these systems to the provided `set`.
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemConfigs {
        self.into_configs().in_set(set)
    }

    /// Run before all systems in `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().before(set)
    }

    /// Run after all systems in `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().after(set)
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
    /// # let mut schedule = Schedule::new();
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
    /// Use [`run_if`](IntoSystemSetConfig::run_if) on a [`SystemSet`] if you want to make sure
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
    /// # let mut schedule = Schedule::new();
    /// # fn a() {}
    /// # fn b() {}
    /// # fn condition() -> bool { true }
    /// # #[derive(SystemSet, Debug, Eq, PartialEq, Hash, Clone, Copy)]
    /// # struct C;
    /// schedule.add_systems((a, b).run_if(condition));
    /// schedule.add_systems((a, b).in_set(C)).configure_set(C.run_if(condition));
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
    fn chain(self) -> SystemConfigs {
        self.into_configs().chain()
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

        self.in_set_inner(set.dyn_clone());

        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.before_inner(set.dyn_clone());
        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.after_inner(set.dyn_clone());
        self
    }

    fn distributive_run_if<M>(mut self, condition: impl Condition<M> + Clone) -> SystemConfigs {
        self.distributive_run_if_inner(condition);
        self
    }

    fn ambiguous_with<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        self.ambiguous_with_inner(set.dyn_clone());
        self
    }

    fn ambiguous_with_all(mut self) -> Self {
        self.ambiguous_with_all_inner();
        self
    }

    fn run_if<M>(mut self, condition: impl Condition<M>) -> SystemConfigs {
        self.run_if_inner(new_condition(condition));
        self
    }

    fn chain(mut self) -> Self {
        match &mut self {
            SystemConfigs::SystemConfig(_) => { /* no op */ }
            SystemConfigs::Configs { chained, .. } => {
                *chained = true;
            }
        }
        self
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
                    chained: false,
                }
            }
        }
    }
}

all_tuples!(impl_system_collection, 1, 20, P, S);

/// A [`SystemSet`] with scheduling metadata.
pub struct SystemSetConfig {
    pub(super) set: BoxedSystemSet,
    pub(super) graph_info: GraphInfo,
    pub(super) conditions: Vec<BoxedCondition>,
}

impl SystemSetConfig {
    fn new(set: BoxedSystemSet) -> Self {
        // system type sets are automatically populated
        // to avoid unintentionally broad changes, they cannot be configured
        assert!(
            set.system_type().is_none(),
            "configuring system type sets is not allowed"
        );

        Self {
            set,
            graph_info: GraphInfo::default(),
            conditions: Vec::new(),
        }
    }
}

/// Types that can be converted into a [`SystemSetConfig`].
///
/// This has been implemented for all types that implement [`SystemSet`] and boxed trait objects.
pub trait IntoSystemSetConfig: Sized {
    /// Convert into a [`SystemSetConfig`].
    #[doc(hidden)]
    fn into_config(self) -> SystemSetConfig;
    /// Add to the provided `set`.
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemSetConfig {
        self.into_config().in_set(set)
    }
    /// Run before all systems in `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().before(set)
    }
    /// Run after all systems in `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().after(set)
    }
    /// Run the systems in this set only if the [`Condition`] is `true`.
    ///
    /// The `Condition` will be evaluated at most once (per schedule run),
    /// the first time a system in this set prepares to run.
    fn run_if<M>(self, condition: impl Condition<M>) -> SystemSetConfig {
        self.into_config().run_if(condition)
    }
    /// Suppress warnings and errors that would result from systems in this set having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().ambiguous_with(set)
    }
    /// Suppress warnings and errors that would result from systems in this set having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> SystemSetConfig {
        self.into_config().ambiguous_with_all()
    }
}

impl<S: SystemSet> IntoSystemSetConfig for S {
    fn into_config(self) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self))
    }
}

impl IntoSystemSetConfig for BoxedSystemSet {
    fn into_config(self) -> SystemSetConfig {
        SystemSetConfig::new(self)
    }
}

impl IntoSystemSetConfig for SystemSetConfig {
    fn into_config(self) -> Self {
        self
    }

    #[track_caller]
    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            set.system_type().is_none(),
            "adding arbitrary systems to a system type set is not allowed"
        );
        self.graph_info.sets.push(Box::new(set));
        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        self.graph_info.dependencies.push(Dependency::new(
            DependencyKind::Before,
            Box::new(set.into_system_set()),
        ));
        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        self.graph_info.dependencies.push(Dependency::new(
            DependencyKind::After,
            Box::new(set.into_system_set()),
        ));
        self
    }

    fn run_if<M>(mut self, condition: impl Condition<M>) -> Self {
        self.conditions.push(new_condition(condition));
        self
    }

    fn ambiguous_with<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        ambiguous_with(&mut self.graph_info, Box::new(set.into_system_set()));
        self
    }

    fn ambiguous_with_all(mut self) -> Self {
        self.graph_info.ambiguous_with = Ambiguity::IgnoreAll;
        self
    }
}

/// A collection of [`SystemSetConfig`].
pub struct SystemSetConfigs {
    pub(super) sets: Vec<SystemSetConfig>,
    /// If `true`, adds `before -> after` ordering constraints between the successive elements.
    pub(super) chained: bool,
}

/// Types that can convert into a [`SystemSetConfigs`].
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

    /// Run before all systems in `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().before(set)
    }

    /// Run after all systems in `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().after(set)
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
        for config in &mut self.sets {
            config.graph_info.sets.push(set.dyn_clone());
        }

        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in &mut self.sets {
            config
                .graph_info
                .dependencies
                .push(Dependency::new(DependencyKind::Before, set.dyn_clone()));
        }

        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in &mut self.sets {
            config
                .graph_info
                .dependencies
                .push(Dependency::new(DependencyKind::After, set.dyn_clone()));
        }

        self
    }

    fn ambiguous_with<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in &mut self.sets {
            ambiguous_with(&mut config.graph_info, set.dyn_clone());
        }

        self
    }

    fn ambiguous_with_all(mut self) -> Self {
        for config in &mut self.sets {
            config.graph_info.ambiguous_with = Ambiguity::IgnoreAll;
        }

        self
    }

    fn chain(mut self) -> Self {
        self.chained = true;
        self
    }
}

macro_rules! impl_system_set_collection {
    ($($set: ident),*) => {
        impl<$($set: IntoSystemSetConfig),*> IntoSystemSetConfigs for ($($set,)*)
        {
            #[allow(non_snake_case)]
            fn into_configs(self) -> SystemSetConfigs {
                let ($($set,)*) = self;
                SystemSetConfigs {
                    sets: vec![$($set.into_config(),)*],
                    chained: false,
                }
            }
        }
    }
}

all_tuples!(impl_system_set_collection, 0, 15, S);
