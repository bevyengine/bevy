use bevy_ecs_macros::all_tuples;
use bevy_utils::default;

use crate::{
    schedule_v3::{
        condition::{BoxedCondition, Condition},
        graph_utils::{Ambiguity, Dependency, DependencyKind, GraphInfo},
        set::{BoxedSystemSet, IntoSystemSet, SystemSet},
    },
    system::{BoxedSystem, IntoSystem, System},
};

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
            !set.is_system_type(),
            "configuring system type sets is not allowed"
        );

        Self {
            set,
            graph_info: GraphInfo {
                sets: Vec::new(),
                dependencies: Vec::new(),
                ambiguous_with: default(),
            },
            conditions: Vec::new(),
        }
    }
}

/// A [`System`] with scheduling metadata.
pub struct SystemConfig {
    pub(super) system: BoxedSystem,
    pub(super) graph_info: GraphInfo,
    pub(super) conditions: Vec<BoxedCondition>,
}

impl SystemConfig {
    fn new(system: BoxedSystem) -> Self {
        // include system in its default sets
        let sets = system.default_system_sets().into_iter().collect();
        Self {
            system,
            graph_info: GraphInfo {
                sets,
                dependencies: Vec::new(),
                ambiguous_with: default(),
            },
            conditions: Vec::new(),
        }
    }
}

fn new_condition<P>(condition: impl Condition<P>) -> BoxedCondition {
    let condition_system = IntoSystem::into_system(condition);
    assert!(
        condition_system.is_send(),
        "Condition `{}` accesses thread-local resources. This is not currently supported.",
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

/// Types that can be converted into a [`SystemSetConfig`].
///
/// This has been implemented for all types that implement [`SystemSet`] and boxed trait objects.
pub trait IntoSystemSetConfig: sealed::IntoSystemSetConfig {
    /// Convert into a [`SystemSetConfig`].
    #[doc(hidden)]
    fn into_config(self) -> SystemSetConfig;
    /// Add to the provided `set`.
    fn in_set(self, set: impl SystemSet) -> SystemSetConfig;
    /// Run before all systems in `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Run after all systems in `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Run the systems in this set only if the [`Condition`] is `true`.
    ///
    /// The `Condition` will be evaluated at most once (per schedule run),
    /// the first time a system in this set prepares to run.
    fn run_if<P>(self, condition: impl Condition<P>) -> SystemSetConfig;
    /// Suppress warnings and errors that would result from systems in this set having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Suppress warnings and errors that would result from systems in this set having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> SystemSetConfig;
}

impl<S> IntoSystemSetConfig for S
where
    S: SystemSet + sealed::IntoSystemSetConfig,
{
    fn into_config(self) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self))
    }

    fn in_set(self, set: impl SystemSet) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self)).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self)).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self)).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self)).run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self)).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self)).ambiguous_with_all()
    }
}

impl IntoSystemSetConfig for BoxedSystemSet {
    fn into_config(self) -> SystemSetConfig {
        SystemSetConfig::new(self)
    }

    fn in_set(self, set: impl SystemSet) -> SystemSetConfig {
        SystemSetConfig::new(self).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        SystemSetConfig::new(self).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        SystemSetConfig::new(self).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemSetConfig {
        SystemSetConfig::new(self).run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        SystemSetConfig::new(self).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemSetConfig {
        SystemSetConfig::new(self).ambiguous_with_all()
    }
}

impl IntoSystemSetConfig for SystemSetConfig {
    fn into_config(self) -> Self {
        self
    }

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            !set.is_system_type(),
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

    fn run_if<P>(mut self, condition: impl Condition<P>) -> Self {
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

/// Types that can be converted into a [`SystemConfig`].
///
/// This has been implemented for boxed [`System<In=(), Out=()>`](crate::system::System)
/// trait objects and all functions that turn into such.
pub trait IntoSystemConfig<Params>: sealed::IntoSystemConfig<Params> {
    /// Convert into a [`SystemConfig`].
    #[doc(hidden)]
    fn into_config(self) -> SystemConfig;
    /// Add to `set` membership.
    fn in_set(self, set: impl SystemSet) -> SystemConfig;
    /// Run before all systems in `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig;
    /// Run after all systems in `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig;
    /// Run only if the [`Condition`] is `true`.
    ///
    /// The `Condition` will be evaluated at most once (per schedule run),
    /// when the system prepares to run.
    fn run_if<P>(self, condition: impl Condition<P>) -> SystemConfig;
    /// Suppress warnings and errors that would result from this system having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig;
    /// Suppress warnings and errors that would result from this system having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> SystemConfig;
}

impl<Params, F> IntoSystemConfig<Params> for F
where
    F: IntoSystem<(), (), Params> + sealed::IntoSystemConfig<Params>,
{
    fn into_config(self) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self)))
    }

    fn in_set(self, set: impl SystemSet) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self))).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self))).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self))).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self))).run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self))).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self))).ambiguous_with_all()
    }
}

impl IntoSystemConfig<()> for BoxedSystem<(), ()> {
    fn into_config(self) -> SystemConfig {
        SystemConfig::new(self)
    }

    fn in_set(self, set: impl SystemSet) -> SystemConfig {
        SystemConfig::new(self).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        SystemConfig::new(self).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        SystemConfig::new(self).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemConfig {
        SystemConfig::new(self).run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        SystemConfig::new(self).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemConfig {
        SystemConfig::new(self).ambiguous_with_all()
    }
}

impl IntoSystemConfig<()> for SystemConfig {
    fn into_config(self) -> Self {
        self
    }

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            !set.is_system_type(),
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

    fn run_if<P>(mut self, condition: impl Condition<P>) -> Self {
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

// only `System<In=(), Out=()>` system objects can be scheduled
mod sealed {
    use crate::{
        schedule_v3::{BoxedSystemSet, SystemSet},
        system::{BoxedSystem, IntoSystem},
    };

    use super::{SystemConfig, SystemSetConfig};

    pub trait IntoSystemConfig<Params> {}

    impl<Params, F: IntoSystem<(), (), Params>> IntoSystemConfig<Params> for F {}

    impl IntoSystemConfig<()> for BoxedSystem<(), ()> {}

    impl IntoSystemConfig<()> for SystemConfig {}

    pub trait IntoSystemSetConfig {}

    impl<S: SystemSet> IntoSystemSetConfig for S {}

    impl IntoSystemSetConfig for BoxedSystemSet {}

    impl IntoSystemSetConfig for SystemSetConfig {}
}

/// A collection of [`SystemConfig`].
pub struct SystemConfigs {
    pub(super) systems: Vec<SystemConfig>,
    /// If `true`, adds `before -> after` ordering constraints between the successive elements.
    pub(super) chained: bool,
}

/// Types that can convert into a [`SystemConfigs`].
pub trait IntoSystemConfigs<Params>
where
    Self: Sized,
{
    /// Convert into a [`SystemConfigs`].
    #[doc(hidden)]
    fn into_configs(self) -> SystemConfigs;

    /// Add these systems to the provided `set`.
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

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            !set.is_system_type(),
            "adding arbitrary systems to a system type set is not allowed"
        );
        for config in &mut self.systems {
            config.graph_info.sets.push(set.dyn_clone());
        }

        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in &mut self.systems {
            config
                .graph_info
                .dependencies
                .push(Dependency::new(DependencyKind::Before, set.dyn_clone()));
        }

        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in &mut self.systems {
            config
                .graph_info
                .dependencies
                .push(Dependency::new(DependencyKind::After, set.dyn_clone()));
        }

        self
    }

    fn ambiguous_with<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in &mut self.systems {
            ambiguous_with(&mut config.graph_info, set.dyn_clone());
        }

        self
    }

    fn ambiguous_with_all(mut self) -> Self {
        for config in &mut self.systems {
            config.graph_info.ambiguous_with = Ambiguity::IgnoreAll;
        }

        self
    }

    fn chain(mut self) -> Self {
        self.chained = true;
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

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            !set.is_system_type(),
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

macro_rules! impl_system_collection {
    ($(($param: ident, $sys: ident)),*) => {
        impl<$($param, $sys),*> IntoSystemConfigs<($($param,)*)> for ($($sys,)*)
        where
            $($sys: IntoSystemConfig<$param>),*
        {
            #[allow(non_snake_case)]
            fn into_configs(self) -> SystemConfigs {
                let ($($sys,)*) = self;
                SystemConfigs {
                    systems: vec![$($sys.into_config(),)*],
                    chained: false,
                }
            }
        }
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

all_tuples!(impl_system_collection, 0, 15, P, S);
all_tuples!(impl_system_set_collection, 0, 15, S);
