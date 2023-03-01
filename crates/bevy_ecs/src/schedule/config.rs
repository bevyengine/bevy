use bevy_utils::all_tuples;

use crate::{
    schedule::{
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
            set.system_type().is_none(),
            "configuring system type sets is not allowed"
        );

        Self {
            set,
            graph_info: GraphInfo::system_set(),
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
        let mut graph_info = GraphInfo::system();
        graph_info.sets = sets;
        Self {
            system,
            graph_info,
            conditions: Vec::new(),
        }
    }
}

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

/// Types that can be converted into a [`SystemSetConfig`].
///
/// This has been implemented for all types that implement [`SystemSet`] and boxed trait objects.
pub trait IntoSystemSetConfig {
    /// Convert into a [`SystemSetConfig`].
    #[doc(hidden)]
    fn into_config(self) -> SystemSetConfig;
    /// Add to the provided `set`.
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemSetConfig;
    /// Add to the provided "base" `set`. For more information on base sets, see [`SystemSet::is_base`].
    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> SystemSetConfig;
    /// Add this set to the schedules's default base set.
    fn in_default_base_set(self) -> SystemSetConfig;
    /// Run before all systems in `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Run after all systems in `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Run the systems in this set only if the [`Condition`] is `true`.
    ///
    /// The `Condition` will be evaluated at most once (per schedule run),
    /// the first time a system in this set prepares to run.
    fn run_if<M>(self, condition: impl Condition<M>) -> SystemSetConfig;
    /// Suppress warnings and errors that would result from systems in this set having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Suppress warnings and errors that would result from systems in this set having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> SystemSetConfig;
}

impl<S: SystemSet> IntoSystemSetConfig for S {
    fn into_config(self) -> SystemSetConfig {
        SystemSetConfig::new(Box::new(self))
    }

    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemSetConfig {
        self.into_config().in_set(set)
    }

    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> SystemSetConfig {
        self.into_config().in_base_set(set)
    }

    fn in_default_base_set(self) -> SystemSetConfig {
        self.into_config().in_default_base_set()
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().after(set)
    }

    fn run_if<M>(self, condition: impl Condition<M>) -> SystemSetConfig {
        self.into_config().run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemSetConfig {
        self.into_config().ambiguous_with_all()
    }
}

impl IntoSystemSetConfig for BoxedSystemSet {
    fn into_config(self) -> SystemSetConfig {
        SystemSetConfig::new(self)
    }

    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemSetConfig {
        self.into_config().in_set(set)
    }

    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> SystemSetConfig {
        self.into_config().in_base_set(set)
    }

    fn in_default_base_set(self) -> SystemSetConfig {
        self.into_config().in_default_base_set()
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().after(set)
    }

    fn run_if<M>(self, condition: impl Condition<M>) -> SystemSetConfig {
        self.into_config().run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        self.into_config().ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemSetConfig {
        self.into_config().ambiguous_with_all()
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
        assert!(
            !set.is_base(),
            "Sets cannot be added to 'base' system sets using 'in_set'. Use 'in_base_set' instead."
        );
        assert!(
            !self.set.is_base(),
            "Base system sets cannot be added to other sets."
        );
        self.graph_info.sets.push(Box::new(set));
        self
    }

    #[track_caller]
    fn in_base_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            set.system_type().is_none(),
            "System type sets cannot be base sets."
        );
        assert!(
            set.is_base(),
            "Sets cannot be added to normal sets using 'in_base_set'. Use 'in_set' instead."
        );
        assert!(
            !self.set.is_base(),
            "Base system sets cannot be added to other sets."
        );
        self.graph_info.set_base_set(Box::new(set));
        self
    }

    fn in_default_base_set(mut self) -> SystemSetConfig {
        self.graph_info.add_default_base_set = true;
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

/// Types that can be converted into a [`SystemConfig`].
///
/// This has been implemented for boxed [`System<In=(), Out=()>`](crate::system::System)
/// trait objects and all functions that turn into such.
pub trait IntoSystemConfig<Marker, Config = SystemConfig> {
    /// Convert into a [`SystemConfig`].
    #[doc(hidden)]
    fn into_config(self) -> Config;
    /// Add to `set` membership.
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> Config;
    /// Add to the provided "base" `set`. For more information on base sets, see [`SystemSet::is_base`].
    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> Config;
    /// Don't add this system to the schedules's default set.
    fn no_default_base_set(self) -> Config;
    /// Run before all systems in `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> Config;
    /// Run after all systems in `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> Config;
    /// Run only if the [`Condition`] is `true`.
    ///
    /// The `Condition` will be evaluated at most once (per schedule run),
    /// when the system prepares to run.
    fn run_if<M>(self, condition: impl Condition<M>) -> Config;
    /// Suppress warnings and errors that would result from this system having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> Config;
    /// Suppress warnings and errors that would result from this system having ambiguities
    /// (conflicting access but indeterminate order) with any other system.
    fn ambiguous_with_all(self) -> Config;
}

impl<Marker, F> IntoSystemConfig<Marker> for F
where
    F: IntoSystem<(), (), Marker>,
{
    fn into_config(self) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self)))
    }

    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemConfig {
        self.into_config().in_set(set)
    }

    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> SystemConfig {
        self.into_config().in_base_set(set)
    }

    fn no_default_base_set(self) -> SystemConfig {
        self.into_config().no_default_base_set()
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        self.into_config().before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        self.into_config().after(set)
    }

    fn run_if<M>(self, condition: impl Condition<M>) -> SystemConfig {
        self.into_config().run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        self.into_config().ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemConfig {
        self.into_config().ambiguous_with_all()
    }
}

impl IntoSystemConfig<()> for BoxedSystem<(), ()> {
    fn into_config(self) -> SystemConfig {
        SystemConfig::new(self)
    }

    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemConfig {
        self.into_config().in_set(set)
    }

    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> SystemConfig {
        self.into_config().in_base_set(set)
    }

    fn no_default_base_set(self) -> SystemConfig {
        self.into_config().no_default_base_set()
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        self.into_config().before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        self.into_config().after(set)
    }

    fn run_if<M>(self, condition: impl Condition<M>) -> SystemConfig {
        self.into_config().run_if(condition)
    }

    fn ambiguous_with<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        self.into_config().ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemConfig {
        self.into_config().ambiguous_with_all()
    }
}

impl IntoSystemConfig<()> for SystemConfig {
    fn into_config(self) -> Self {
        self
    }

    #[track_caller]
    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            set.system_type().is_none(),
            "adding arbitrary systems to a system type set is not allowed"
        );
        assert!(
            !set.is_base(),
            "Systems cannot be added to 'base' system sets using 'in_set'. Use 'in_base_set' instead."
        );
        self.graph_info.sets.push(Box::new(set));
        self
    }

    #[track_caller]
    fn in_base_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            set.system_type().is_none(),
            "System type sets cannot be base sets."
        );
        assert!(
            set.is_base(),
            "Systems cannot be added to normal sets using 'in_base_set'. Use 'in_set' instead."
        );
        self.graph_info.set_base_set(Box::new(set));
        self
    }

    fn no_default_base_set(mut self) -> SystemConfig {
        self.graph_info.add_default_base_set = false;
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

/// A collection of [`SystemConfig`].
pub struct SystemConfigs {
    pub(super) systems: Vec<SystemConfig>,
    /// If `true`, adds `before -> after` ordering constraints between the successive elements.
    pub(super) chained: bool,
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

    /// Add these systems to the provided "base" `set`. For more information on base sets, see [`SystemSet::is_base`].
    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> SystemConfigs {
        self.into_configs().in_base_set(set)
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
    /// This is equivalent to calling [`run_if`](IntoSystemConfig::run_if) on each individual
    /// system, as shown below:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut app = Schedule::new();
    /// # fn a() {}
    /// # fn b() {}
    /// # fn condition() -> bool { true }
    /// app.add_systems((a, b).distributive_run_if(condition));
    /// app.add_systems((a.run_if(condition), b.run_if(condition)));
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
        assert!(
            !set.is_base(),
            "Systems cannot be added to 'base' system sets using 'in_set'. Use 'in_base_set' instead."
        );
        for config in &mut self.systems {
            config.graph_info.sets.push(set.dyn_clone());
        }

        self
    }

    #[track_caller]
    fn in_base_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            set.system_type().is_none(),
            "System type sets cannot be base sets."
        );
        assert!(
            set.is_base(),
            "Systems cannot be added to normal sets using 'in_base_set'. Use 'in_set' instead."
        );
        for config in &mut self.systems {
            config.graph_info.set_base_set(set.dyn_clone());
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

    fn distributive_run_if<M>(mut self, condition: impl Condition<M> + Clone) -> SystemConfigs {
        for config in &mut self.systems {
            config.conditions.push(new_condition(condition.clone()));
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
    #[track_caller]
    fn in_set(self, set: impl SystemSet) -> SystemSetConfigs {
        self.into_configs().in_set(set)
    }

    /// Add these system sets to the provided "base" `set`. For more information on base sets, see [`SystemSet::is_base`].
    #[track_caller]
    fn in_base_set(self, set: impl SystemSet) -> SystemSetConfigs {
        self.into_configs().in_base_set(set)
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
        assert!(
            !set.is_base(),
            "Sets cannot be added to 'base' system sets using 'in_set'. Use 'in_base_set' instead."
        );
        for config in &mut self.sets {
            assert!(
                !config.set.is_base(),
                "Base system sets cannot be added to other sets."
            );
            config.graph_info.sets.push(set.dyn_clone());
        }

        self
    }

    #[track_caller]
    fn in_base_set(mut self, set: impl SystemSet) -> Self {
        assert!(
            set.system_type().is_none(),
            "System type sets cannot be base sets."
        );
        assert!(
            set.is_base(),
            "Sets cannot be added to normal sets using 'in_base_set'. Use 'in_set' instead."
        );
        for config in &mut self.sets {
            assert!(
                !config.set.is_base(),
                "Base system sets cannot be added to other sets."
            );
            config.graph_info.set_base_set(set.dyn_clone());
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
