use crate::schedule::{
    AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystemLabel, IntoRunCriterionDescriptor,
    RunCriterionDescriptor, SystemDescriptor, SystemLabel,
};

/// A builder for describing several systems at the same time.
pub struct SystemSet {
    pub(crate) systems: Vec<SystemDescriptor>,
    pub(crate) run_criterion: Option<RunCriterionDescriptor>,
    pub(crate) labels: Vec<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
}

impl Default for SystemSet {
    fn default() -> SystemSet {
        SystemSet {
            systems: Vec::new(),
            run_criterion: None,
            labels: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            ambiguity_sets: Vec::new(),
        }
    }
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_system(mut self, system: impl Into<SystemDescriptor>) -> Self {
        self.systems.push(system.into());
        self
    }

    pub fn with_run_criterion<Marker>(
        mut self,
        run_criteria: impl IntoRunCriterionDescriptor<Marker>,
    ) -> Self {
        self.run_criterion = Some(run_criteria.into());
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

    pub fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.ambiguity_sets.push(Box::new(set));
        self
    }

    pub(crate) fn bake(self) -> (Option<RunCriterionDescriptor>, Vec<SystemDescriptor>) {
        let SystemSet {
            mut systems,
            run_criterion: run_criteria,
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
