use crate::{
    component::Component,
    prelude::IntoSystem,
    schedule::{RunCriteriaDescriptorOrLabel, State},
    system::{BoxedSystem, RunCriteraConfig, SystemConfig},
};
use std::{fmt::Debug, hash::Hash};

/// A builder for describing several systems at the same time.
#[derive(Default)]
pub struct SystemSet {
    pub(crate) systems: Vec<BoxedSystem>,
    pub(crate) config: SystemConfig,
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

    pub fn with_system<Param>(mut self, system: impl IntoSystem<(), (), Param>) -> Self {
        self.systems.push(Box::new(system.system()));
        self
    }

    pub(crate) fn bake(self) -> (Option<RunCriteriaDescriptorOrLabel>, Vec<BoxedSystem>) {
        let SystemSet {
            mut systems,
            config,
        } = self;
        for system in &mut systems {
            system
                .config_mut()
                .labels
                .extend(config.labels.iter().cloned());
            system
                .config_mut()
                .before
                .extend(config.before.iter().cloned());
            system
                .config_mut()
                .after
                .extend(config.after.iter().cloned());
            system
                .config_mut()
                .ambiguity_sets
                .extend(config.ambiguity_sets.iter().cloned());
        }
        (config.run_criteria, systems)
    }

    pub(crate) fn config(&self) -> &SystemConfig {
        &self.config
    }
    pub(crate) fn config_mut(&mut self) -> &mut SystemConfig {
        &mut self.config
    }
}
