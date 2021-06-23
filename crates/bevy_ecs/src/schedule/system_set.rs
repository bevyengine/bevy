use crate::{
    component::Component,
    schedule::{
        AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystemLabel, IntoRunCriteria,
        RunCriteriaDescriptorOrLabel, State, SystemDescriptor, SystemLabel,
    },
};
use std::{
    fmt::Debug,
    hash::Hash,
    sync::atomic::{AtomicU32, Ordering},
};

use super::IntoSystemDescriptor;

/// A builder for describing several systems at the same time.
pub struct SystemSet {
    pub(crate) systems: Vec<SystemDescriptor>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
    sequential: bool,
}

impl Default for SystemSet {
    fn default() -> SystemSet {
        SystemSet {
            systems: Vec::new(),
            run_criteria: None,
            labels: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            ambiguity_sets: Vec::new(),
            sequential: false,
        }
    }
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn on_update<T>(s: T) -> SystemSet
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        Self::new().with_run_criteria(State::<T>::on_update(s))
    }

    pub fn on_inactive_update<T>(s: T) -> SystemSet
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        Self::new().with_run_criteria(State::<T>::on_inactive_update(s))
    }

    pub fn on_in_stack_update<T>(s: T) -> SystemSet
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        Self::new().with_run_criteria(State::<T>::on_in_stack_update(s))
    }

    pub fn on_enter<T>(s: T) -> SystemSet
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        Self::new().with_run_criteria(State::<T>::on_enter(s))
    }

    pub fn on_exit<T>(s: T) -> SystemSet
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        Self::new().with_run_criteria(State::<T>::on_exit(s))
    }

    pub fn on_pause<T>(s: T) -> SystemSet
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        Self::new().with_run_criteria(State::<T>::on_pause(s))
    }

    pub fn on_resume<T>(s: T) -> SystemSet
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        Self::new().with_run_criteria(State::<T>::on_resume(s))
    }

    pub fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.ambiguity_sets.push(Box::new(set));
        self
    }

    pub fn with_system<Params>(mut self, system: impl IntoSystemDescriptor<Params>) -> Self {
        self.systems.push(system.into_descriptor());
        self
    }

    pub fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.run_criteria = Some(run_criteria.into());
        self
    }

    pub fn label(mut self, label: impl SystemLabel) -> Self {
        self.labels.push(Box::new(label));
        self
    }

    pub fn before(mut self, label: impl SystemLabel) -> Self {
        self.before.push(Box::new(label));
        self
    }

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
            sequential,
        } = self;

        if sequential {
            Self::sequentialize(&mut systems);
        }

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

    fn sequentialize(systems: &mut Vec<SystemDescriptor>) {
        let start = NEXT_SEQUENCE_ID.fetch_add(systems.len() as u32, Ordering::Relaxed);
        let mut last_label: Option<SequenceId> = None;
        for (idx, descriptor) in systems.iter_mut().enumerate() {
            let label = SequenceId(start.wrapping_add(idx as u32));
            match descriptor {
                SystemDescriptor::Parallel(descriptor) => {
                    descriptor.labels.push(label.dyn_clone());
                    if let Some(ref after) = last_label {
                        descriptor.after.push(after.dyn_clone());
                    }
                }
                SystemDescriptor::Exclusive(descriptor) => {
                    descriptor.labels.push(label.dyn_clone());
                    if let Some(ref after) = last_label {
                        descriptor.after.push(after.dyn_clone());
                    }
                }
            }
            last_label = Some(label);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SequenceId(u32);

impl SystemLabel for SequenceId {
    fn dyn_clone(&self) -> Box<dyn SystemLabel> {
        Box::new(<SequenceId>::clone(self))
    }
}
