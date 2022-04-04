use crate::schedule::{
    AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystemLabel, IntoRunCriteria,
    RunCriteriaDescriptorOrLabel, State, StateData, SystemDescriptor, SystemLabel,
};

use super::IntoSystemDescriptor;

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
    pub fn before(mut self, label: impl SystemLabel) -> Self {
        self.before.push(Box::new(label));
        self
    }

    #[must_use]
    pub fn after(mut self, label: impl SystemLabel) -> Self {
        self.after.push(Box::new(label));
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
