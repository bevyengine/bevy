use crate::{
    component::Component,
    prelude::{ExclusiveSystem, IntoExclusiveSystem, IntoSystem},
    schedule::{RunCriteriaDescriptorOrLabel, State},
    system::{BoxedExclusiveSystem, BoxedSystem, RunCriteraConfig, SystemConfig},
};
use std::{fmt::Debug, hash::Hash};

/// A builder for describing several systems at the same time.
#[derive(Default)]
pub struct SystemSet {
    pub systems: Vec<BoxedSystem>,
    pub exclusive_systems: Vec<BoxedExclusiveSystem>,
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

    pub fn with_system<Params>(mut self, system: impl IntoSystem<(), (), Params>) -> Self {
        self.systems.push(Box::new(system.system()));
        self
    }

    pub fn with_exclusive<Params, SystemType>(
        mut self,
        system: impl IntoExclusiveSystem<Params, SystemType>,
    ) -> Self
    where
        SystemType: ExclusiveSystem,
    {
        self.exclusive_systems
            .push(Box::new(system.exclusive_system()));
        self
    }

    pub(crate) fn bake(
        self,
    ) -> (
        Option<RunCriteriaDescriptorOrLabel>,
        Vec<BoxedSystem>,
        Vec<BoxedExclusiveSystem>,
    ) {
        let SystemSet {
            mut systems,
            mut exclusive_systems,
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
        for system in &mut exclusive_systems {
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
        (config.run_criteria, systems, exclusive_systems)
    }

    pub fn config(&self) -> &SystemConfig {
        &self.config
    }
    pub fn config_mut(&mut self) -> &mut SystemConfig {
        &mut self.config
    }
}
