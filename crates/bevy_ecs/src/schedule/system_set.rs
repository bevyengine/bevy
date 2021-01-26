use crate::{ExclusiveSystemDescriptor, ParallelSystemDescriptor, RunCriteria, ShouldRun, System};

pub struct SystemSet {
    pub(crate) run_criteria: RunCriteria,
    pub(crate) exclusive_descriptors: Vec<ExclusiveSystemDescriptor>,
    pub(crate) parallel_descriptors: Vec<ParallelSystemDescriptor>,
}

impl Default for SystemSet {
    fn default() -> SystemSet {
        SystemSet {
            run_criteria: Default::default(),
            exclusive_descriptors: vec![],
            parallel_descriptors: vec![],
        }
    }
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.add_run_criteria(system);
        self
    }

    pub fn add_run_criteria<S: System<In = (), Out = ShouldRun>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.run_criteria.set(Box::new(system));
        self
    }

    pub fn with_system(mut self, system: impl Into<ParallelSystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn with_exclusive_system(mut self, system: impl Into<ExclusiveSystemDescriptor>) -> Self {
        self.add_exclusive_system(system);
        self
    }

    pub fn add_system(&mut self, system: impl Into<ParallelSystemDescriptor>) -> &mut Self {
        self.parallel_descriptors.push(system.into());
        self
    }

    pub fn add_exclusive_system(
        &mut self,
        system: impl Into<ExclusiveSystemDescriptor>,
    ) -> &mut Self {
        self.exclusive_descriptors.push(system.into());
        self
    }
}
