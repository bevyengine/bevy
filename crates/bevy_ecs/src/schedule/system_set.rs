use crate::{RunCriteria, ShouldRun, System, SystemDescriptor};

/// Describes a group of systems sharing one run criterion.
pub struct SystemSet {
    pub(crate) run_criteria: RunCriteria,
    pub(crate) descriptors: Vec<SystemDescriptor>,
}

impl Default for SystemSet {
    fn default() -> SystemSet {
        SystemSet {
            run_criteria: Default::default(),
            descriptors: vec![],
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

    pub fn with_system(mut self, system: impl Into<SystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn add_system(&mut self, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.descriptors.push(system.into());
        self
    }
}
