use crate::schedule::{
    AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystemLabel, IntoRunCriteria,
    RunCriteriaDescriptorOrLabel, State, StateData, SystemDescriptor, SystemLabel,
};

use super::IntoSystemDescriptor;
use std::{
    fmt::Debug,
    hash::Hash,
    sync::atomic::{AtomicU32, Ordering},
};

static NEXT_SEQUENCE_ID: AtomicU32 = AtomicU32::new(0);

/// A builder for describing several systems at the same time.
#[derive(Default)]
pub struct SystemSet {
    pub(crate) systems: Vec<SystemDescriptor>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
    sequential: bool,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;

    fn dummy_system() {}

    fn labels(system: &SystemDescriptor) -> &Vec<Box<dyn SystemLabel>> {
        match system {
            SystemDescriptor::Parallel(descriptor) => &descriptor.labels,
            SystemDescriptor::Exclusive(descriptor) => &descriptor.labels,
        }
    }

    fn after(system: &SystemDescriptor) -> &Vec<Box<dyn SystemLabel>> {
        match system {
            SystemDescriptor::Parallel(descriptor) => &descriptor.after,
            SystemDescriptor::Exclusive(descriptor) => &descriptor.after,
        }
    }

    #[test]
    pub fn sequential_adds_labels() {
        let system_set = SystemSet::new()
            .as_sequential()
            .with_system(dummy_system.system())
            .with_system(dummy_system.system())
            .with_system(dummy_system.system());
        let (_, systems) = system_set.bake();

        assert_eq!(systems.len(), 3);
        assert_eq!(labels(&systems[0]), &vec![SequenceId(0).dyn_clone()]);
        assert_eq!(labels(&systems[1]), &vec![SequenceId(1).dyn_clone()]);
        assert_eq!(labels(&systems[2]), &vec![SequenceId(2).dyn_clone()]);
        assert_eq!(after(&systems[0]), &vec![]);
        assert_eq!(after(&systems[1]), &vec![SequenceId(0).dyn_clone()]);
        assert_eq!(after(&systems[2]), &vec![SequenceId(1).dyn_clone()]);
    }

    #[test]
    pub fn non_sequential_has_no_labels_by_default() {
        let system_set = SystemSet::new()
            .with_system(dummy_system.system())
            .with_system(dummy_system.system())
            .with_system(dummy_system.system());
        let (_, systems) = system_set.bake();

        assert_eq!(systems.len(), 3);
        assert_eq!(labels(&systems[0]), &vec![]);
        assert_eq!(labels(&systems[1]), &vec![]);
        assert_eq!(labels(&systems[2]), &vec![]);
        assert_eq!(after(&systems[0]), &vec![]);
        assert_eq!(after(&systems[1]), &vec![]);
        assert_eq!(after(&systems[2]), &vec![]);
    }
}
