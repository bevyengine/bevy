use bevy_ecs_macros::impl_into_linked_system_set;

use crate::schedule::{
    AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystemLabel, IntoRunCriteria,
    IntoSystemDescriptor, RunCriteriaDescriptorOrLabel, State, StateData, SystemDescriptor,
    SystemLabel,
};
use crate::system::{AsSystemLabel, SystemParam, SystemParamFunction};

use super::ParallelSystemDescriptorCoercion;

/// A builder for describing several systems at the same time.
#[derive(Default)]
pub struct SystemSet {
    pub(crate) systems: Vec<SystemDescriptor>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn on_update<T>(s: T) -> SystemSet
    where
        T: StateData,
    {
        Self::new().with_run_criteria(State::<T>::on_update(s))
    }

    pub fn on_inactive_update<T>(s: T) -> SystemSet
    where
        T: StateData,
    {
        Self::new().with_run_criteria(State::<T>::on_inactive_update(s))
    }

    pub fn on_in_stack_update<T>(s: T) -> SystemSet
    where
        T: StateData,
    {
        Self::new().with_run_criteria(State::<T>::on_in_stack_update(s))
    }

    pub fn on_enter<T>(s: T) -> SystemSet
    where
        T: StateData,
    {
        Self::new().with_run_criteria(State::<T>::on_enter(s))
    }

    pub fn on_exit<T>(s: T) -> SystemSet
    where
        T: StateData,
    {
        Self::new().with_run_criteria(State::<T>::on_exit(s))
    }

    pub fn on_pause<T>(s: T) -> SystemSet
    where
        T: StateData,
    {
        Self::new().with_run_criteria(State::<T>::on_pause(s))
    }

    pub fn on_resume<T>(s: T) -> SystemSet
    where
        T: StateData,
    {
        Self::new().with_run_criteria(State::<T>::on_resume(s))
    }

    #[must_use]
    pub fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.ambiguity_sets.push(Box::new(set));
        self
    }

    #[must_use]
    pub fn with_system<Params>(mut self, system: impl IntoSystemDescriptor<Params>) -> Self {
        self.systems.push(system.into_descriptor());
        self
    }

    #[must_use]
    pub fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.run_criteria = Some(run_criteria.into());
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl SystemLabel) -> Self {
        self.labels.push(Box::new(label));
        self
    }

    #[must_use]
    pub fn before<Marker>(mut self, label: impl AsSystemLabel<Marker>) -> Self {
        self.before.push(Box::new(label.as_system_label()));
        self
    }

    #[must_use]
    pub fn after<Marker>(mut self, label: impl AsSystemLabel<Marker>) -> Self {
        self.after.push(Box::new(label.as_system_label()));
        self
    }

    pub(crate) fn bake(self) -> (Option<RunCriteriaDescriptorOrLabel>, Vec<SystemDescriptor>) {
        let SystemSet {
            mut systems,
            run_criteria,
            labels,
            before,
            after,
            ambiguity_sets,
        } = self;
        for descriptor in &mut systems {
            match descriptor {
                SystemDescriptor::Parallel(descriptor) => {
                    descriptor.labels.extend(labels.iter().cloned());
                    descriptor.before.extend(before.iter().cloned());
                    descriptor.after.extend(after.iter().cloned());
                    descriptor
                        .ambiguity_sets
                        .extend(ambiguity_sets.iter().cloned());
                }
                SystemDescriptor::Exclusive(descriptor) => {
                    descriptor.labels.extend(labels.iter().cloned());
                    descriptor.before.extend(before.iter().cloned());
                    descriptor.after.extend(after.iter().cloned());
                    descriptor
                        .ambiguity_sets
                        .extend(ambiguity_sets.iter().cloned());
                }
            }
        }
        (run_criteria, systems)
    }
}

/// A trait that provides the [`.link()`] method to tuples of systems.
///
/// [`.link()`]: IntoLinkedSystemSet::link
pub trait IntoLinkedSystemSet<S, P> {
    /// A helper method to create system sets with automatic
    /// ordering of execution.
    ///
    /// This is only implemented for tuples of 2 to 15 systems.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # fn first_system() {}
    /// # fn second_system() {}
    /// # fn third_system() {}
    /// # fn fourth_system() {}
    ///
    /// // This expression...
    ///
    /// (
    ///     first_system,
    ///     second_system,
    ///     third_system,
    ///     fourth_system,
    /// ).link();
    ///
    /// // ...is equal to:
    ///
    /// SystemSet::new()
    ///     .with_system(first_system)
    ///     .with_system(second_system.after(first_system))
    ///     .with_system(third_system.after(second_system))
    ///     .with_system(fourth_system.after(third_system));
    /// ```
    fn link(self) -> SystemSet;
}

impl_into_linked_system_set!();

/// A trait that provides the [`.then()`] method to systems.
///
/// [`.then()`]: IntoSequentialSystemSet::then
pub trait IntoSequentialSystemSet<S0, S1, P0, P1> {
    /// A helper method to create system sets with automatic
    /// ordering of execution.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # fn first_system() {}
    /// # fn second_system() {}
    ///
    /// // This expression...
    ///
    /// first_system.then(second_system);
    ///
    /// // ...is equal to:
    ///
    /// SystemSet::new()
    ///     .with_system(first_system)
    ///     .with_system(second_system.after(first_system));
    /// ```
    fn then(self, next_system: S1) -> SystemSet;
}

impl<S0, S1, P0, P1> IntoSequentialSystemSet<S0, S1, P0, P1> for S0
where
    P0: SystemParam + 'static,
    P1: SystemParam + 'static,
    S0: SystemParamFunction<(), (), P0, ()> + 'static,
    S1: SystemParamFunction<(), (), P1, ()> + 'static,
{
    fn then(self, next_system: S1) -> SystemSet {
        (self, next_system).link()
    }
}
