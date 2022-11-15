use bevy_utils::prelude::default;
use bevy_utils::HashSet;

use crate::{
    schedule_v3::{
        condition::{BoxedCondition, Condition},
        graph::{Ambiguity, DependencyEdgeKind, GraphInfo},
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

/// A [`System`] with scheduling metadata.
pub struct SystemConfig {
    pub(super) system: BoxedSystem,
    pub(super) graph_info: GraphInfo,
    pub(super) conditions: Vec<BoxedCondition>,
}

pub(super) fn new_set_unchecked(set: BoxedSystemSet) -> SystemSetConfig {
    SystemSetConfig {
        set,
        graph_info: GraphInfo {
            sets: HashSet::new(),
            dependencies: HashSet::new(),
            ambiguous_with: default(),
        },
        conditions: Vec::new(),
    }
}

fn new_set(set: BoxedSystemSet) -> SystemSetConfig {
    assert!(!set.is_system_type());
    new_set_unchecked(set)
}

fn new_system(system: BoxedSystem) -> SystemConfig {
    // include system in its default sets
    let sets = system.default_system_sets().into_iter().collect();
    SystemConfig {
        system,
        graph_info: GraphInfo {
            sets,
            dependencies: HashSet::new(),
            ambiguous_with: default(),
        },
        conditions: Vec::new(),
    }
}

fn new_condition<P>(condition: impl Condition<P>) -> BoxedCondition {
    let condition_system = IntoSystem::into_system(condition);
    assert!(
        condition_system.is_send(),
        "condition accesses thread-local resources (currently not supported)"
    );

    Box::new(condition_system)
}

/// Types that can be converted into a [`SystemSetConfig`].
///
/// This has been implemented for all types that implement [`SystemSet`] and boxed trait objects.
pub trait IntoSystemSetConfig: sealed::IntoSystemSetConfig {
    /// Convert into a [`SystemSetConfig`].
    #[doc(hidden)]
    fn into_config(self) -> SystemSetConfig;
    /// Add to `set` membership.
    fn in_set(self, set: impl SystemSet) -> SystemSetConfig;
    /// Run before all members of `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Run after all members of `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig;
    /// Run only if the [`Condition`] is `true` at the time of execution.
    fn run_if<P>(self, condition: impl Condition<P>) -> SystemSetConfig;
    /// Suppress warnings and errors that would result from "ambiguities" with members of `set`.
    fn ambiguous_with(self, set: impl SystemSet) -> SystemSetConfig;
    /// Suppress warnings and errors that would result from any "ambiguities".
    fn ambiguous_with_all(self) -> SystemSetConfig;
}

impl<S> IntoSystemSetConfig for S
where
    S: SystemSet + sealed::IntoSystemSetConfig,
{
    fn into_config(self) -> SystemSetConfig {
        new_set(self.dyn_clone())
    }

    fn in_set(self, set: impl SystemSet) -> SystemSetConfig {
        new_set(self.dyn_clone()).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        new_set(self.dyn_clone()).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        new_set(self.dyn_clone()).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemSetConfig {
        new_set(self.dyn_clone()).run_if(condition)
    }

    fn ambiguous_with(self, set: impl SystemSet) -> SystemSetConfig {
        new_set(self.dyn_clone()).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemSetConfig {
        new_set(self.dyn_clone()).ambiguous_with_all()
    }
}

impl IntoSystemSetConfig for BoxedSystemSet {
    fn into_config(self) -> SystemSetConfig {
        new_set(self)
    }

    fn in_set(self, set: impl SystemSet) -> SystemSetConfig {
        new_set(self).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        new_set(self).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfig {
        new_set(self).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemSetConfig {
        new_set(self).run_if(condition)
    }

    fn ambiguous_with(self, set: impl SystemSet) -> SystemSetConfig {
        new_set(self).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemSetConfig {
        new_set(self).ambiguous_with_all()
    }
}

impl IntoSystemSetConfig for SystemSetConfig {
    fn into_config(self) -> Self {
        self
    }

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(!set.is_system_type(), "invalid use of system type set");
        self.graph_info.sets.insert(set.dyn_clone());
        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        self.graph_info.dependencies.insert((
            DependencyEdgeKind::Before,
            set.into_system_set().dyn_clone(),
        ));
        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        self.graph_info
            .dependencies
            .insert((DependencyEdgeKind::After, set.into_system_set().dyn_clone()));
        self
    }

    fn run_if<P>(mut self, condition: impl Condition<P>) -> Self {
        self.conditions.push(new_condition(condition));
        self
    }

    fn ambiguous_with(mut self, set: impl SystemSet) -> Self {
        assert!(!set.is_system_type(), "invalid use of system type set");
        match &mut self.graph_info.ambiguous_with {
            detection @ Ambiguity::Check => {
                let mut ambiguous_with = HashSet::new();
                ambiguous_with.insert(set.dyn_clone());
                *detection = Ambiguity::IgnoreWithSet(ambiguous_with);
            }
            Ambiguity::IgnoreWithSet(ambiguous_with) => {
                ambiguous_with.insert(set.dyn_clone());
            }
            Ambiguity::IgnoreAll => (),
        }

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
    /// Run before all members of `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig;
    /// Run after all members of `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig;
    /// Only run if the [`Condition`] is `true` at the time of execution.
    fn run_if<P>(self, condition: impl Condition<P>) -> SystemConfig;
    /// Suppress warnings and errors that would result from "ambiguities" with members of `set`.
    fn ambiguous_with(self, set: impl SystemSet) -> SystemConfig;
    /// Suppress warnings and errors that would result from any "ambiguities".
    fn ambiguous_with_all(self) -> SystemConfig;
}

impl<Params, F> IntoSystemConfig<Params> for F
where
    F: IntoSystem<(), (), Params> + sealed::IntoSystemConfig<Params>,
{
    fn into_config(self) -> SystemConfig {
        new_system(Box::new(IntoSystem::into_system(self)))
    }

    fn in_set(self, set: impl SystemSet) -> SystemConfig {
        new_system(Box::new(IntoSystem::into_system(self))).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        new_system(Box::new(IntoSystem::into_system(self))).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        new_system(Box::new(IntoSystem::into_system(self))).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemConfig {
        new_system(Box::new(IntoSystem::into_system(self))).run_if(condition)
    }

    fn ambiguous_with(self, set: impl SystemSet) -> SystemConfig {
        new_system(Box::new(IntoSystem::into_system(self))).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemConfig {
        new_system(Box::new(IntoSystem::into_system(self))).ambiguous_with_all()
    }
}

impl IntoSystemConfig<()> for BoxedSystem<(), ()> {
    fn into_config(self) -> SystemConfig {
        new_system(self)
    }

    fn in_set(self, set: impl SystemSet) -> SystemConfig {
        new_system(self).in_set(set)
    }

    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        new_system(self).before(set)
    }

    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfig {
        new_system(self).after(set)
    }

    fn run_if<P>(self, condition: impl Condition<P>) -> SystemConfig {
        new_system(self).run_if(condition)
    }

    fn ambiguous_with(self, set: impl SystemSet) -> SystemConfig {
        new_system(self).ambiguous_with(set)
    }

    fn ambiguous_with_all(self) -> SystemConfig {
        new_system(self).ambiguous_with_all()
    }
}

impl IntoSystemConfig<()> for SystemConfig {
    fn into_config(self) -> Self {
        self
    }

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(!set.is_system_type(), "invalid use of system type set");
        self.graph_info.sets.insert(set.dyn_clone());
        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        self.graph_info.dependencies.insert((
            DependencyEdgeKind::Before,
            set.into_system_set().dyn_clone(),
        ));
        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        self.graph_info
            .dependencies
            .insert((DependencyEdgeKind::After, set.into_system_set().dyn_clone()));
        self
    }

    fn run_if<P>(mut self, condition: impl Condition<P>) -> Self {
        self.conditions.push(new_condition(condition));
        self
    }

    fn ambiguous_with(mut self, set: impl SystemSet) -> SystemConfig {
        assert!(!set.is_system_type(), "invalid use of system type set");
        match &mut self.graph_info.ambiguous_with {
            detection @ Ambiguity::Check => {
                let mut ambiguous_with = HashSet::new();
                ambiguous_with.insert(set.dyn_clone());
                *detection = Ambiguity::IgnoreWithSet(ambiguous_with);
            }
            Ambiguity::IgnoreWithSet(ambiguous_with) => {
                ambiguous_with.insert(set.dyn_clone());
            }
            Ambiguity::IgnoreAll => (),
        }

        self
    }

    fn ambiguous_with_all(mut self) -> SystemConfig {
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

    /// Add to `set` membership.
    fn in_set(self, set: impl SystemSet) -> SystemConfigs {
        self.into_configs().in_set(set)
    }

    /// Run before all members of `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().before(set)
    }

    /// Run after all members of `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemConfigs {
        self.into_configs().after(set)
    }

    /// Treat this collection as a sequence.
    ///
    /// Ordering constraints will be applied between the successive collection elements.
    fn chain(self) -> SystemConfigs {
        self.into_configs().chain()
    }
}

impl IntoSystemConfigs<()> for SystemConfigs {
    fn into_configs(self) -> Self {
        self
    }

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(!set.is_system_type(), "invalid use of system type set");
        for config in self.systems.iter_mut() {
            config.graph_info.sets.insert(set.dyn_clone());
        }

        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in self.systems.iter_mut() {
            config
                .graph_info
                .dependencies
                .insert((DependencyEdgeKind::Before, set.dyn_clone()));
        }

        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in self.systems.iter_mut() {
            config
                .graph_info
                .dependencies
                .insert((DependencyEdgeKind::After, set.dyn_clone()));
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

    /// Add to `set` membership.
    fn in_set(self, set: impl SystemSet) -> SystemSetConfigs {
        self.into_configs().in_set(set)
    }

    /// Run before all members of `set`.
    fn before<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().before(set)
    }

    /// Run after all members of `set`.
    fn after<M>(self, set: impl IntoSystemSet<M>) -> SystemSetConfigs {
        self.into_configs().after(set)
    }

    /// Treat this collection as a sequence.
    ///
    /// Ordering constraints will be applied between the successive collection elements.
    fn chain(self) -> SystemSetConfigs {
        self.into_configs().chain()
    }
}

impl IntoSystemSetConfigs for SystemSetConfigs {
    fn into_configs(self) -> Self {
        self
    }

    fn in_set(mut self, set: impl SystemSet) -> Self {
        assert!(!set.is_system_type(), "invalid use of system type set");
        for config in self.sets.iter_mut() {
            config.graph_info.sets.insert(set.dyn_clone());
        }

        self
    }

    fn before<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in self.sets.iter_mut() {
            config
                .graph_info
                .dependencies
                .insert((DependencyEdgeKind::Before, set.dyn_clone()));
        }

        self
    }

    fn after<M>(mut self, set: impl IntoSystemSet<M>) -> Self {
        let set = set.into_system_set();
        for config in self.sets.iter_mut() {
            config
                .graph_info
                .dependencies
                .insert((DependencyEdgeKind::After, set.dyn_clone()));
        }

        self
    }

    fn chain(mut self) -> Self {
        self.chained = true;
        self
    }
}

macro_rules! impl_system_collection {
    ($($param: ident, $sys: ident),*) => {
        impl<$($param, $sys),*> IntoSystemConfigs<($($param),*)> for ($($sys),*)
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
        impl<$($set: IntoSystemSetConfig),*> IntoSystemSetConfigs for ($($set),*)
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

impl_system_collection!(P0, T0, P1, T1);
impl_system_collection!(P0, T0, P1, T1, P2, T2);
impl_system_collection!(P0, T0, P1, T1, P2, T2, P3, T3);
impl_system_collection!(P0, T0, P1, T1, P2, T2, P3, T3, P4, T4);
impl_system_collection!(P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5);
impl_system_collection!(P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6);
impl_system_collection!(P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7);
impl_system_collection!(P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8);
impl_system_collection!(
    P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8, P9, T9
);
impl_system_collection!(
    P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8, P9, T9, P10, T10
);
impl_system_collection!(
    P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8, P9, T9, P10, T10, P11,
    T11
);
impl_system_collection!(
    P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8, P9, T9, P10, T10, P11,
    T11, P12, T12
);
impl_system_collection!(
    P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8, P9, T9, P10, T10, P11,
    T11, P12, T12, P13, T13
);
impl_system_collection!(
    P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8, P9, T9, P10, T10, P11,
    T11, P12, T12, P13, T13, P14, T14
);
impl_system_collection!(
    P0, T0, P1, T1, P2, T2, P3, T3, P4, T4, P5, T5, P6, T6, P7, T7, P8, T8, P9, T9, P10, T10, P11,
    T11, P12, T12, P13, T13, P14, T14, P15, T15
);

impl_system_set_collection!(S0, S1);
impl_system_set_collection!(S0, S1, S2);
impl_system_set_collection!(S0, S1, S2, S3);
impl_system_set_collection!(S0, S1, S2, S3, S4);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8, S9);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14);
impl_system_set_collection!(S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14, S15);
