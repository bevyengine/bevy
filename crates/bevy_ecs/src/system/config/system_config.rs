use crate::{schedule::*, system::InsertionPoint};

/// Each system has one of these. It is updated using the various traits seen in this [directory](super).
#[derive(Default)]
pub struct SystemConfig {
    pub labels: Vec<BoxedSystemLabel>,
    pub before: Vec<BoxedSystemLabel>,
    pub after: Vec<BoxedSystemLabel>,
    pub stage: Option<BoxedStageLabel>,
    pub ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
    pub run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub insertion_point: InsertionPoint,
    pub startup: bool,
}

impl SystemConfig {
    pub(crate) fn add_label(&mut self, label: impl SystemLabel) {
        self.labels.push(Box::new(label));
    }
    pub(crate) fn add_before(&mut self, label: impl SystemLabel) {
        self.before.push(Box::new(label));
    }
    pub(crate) fn add_after(&mut self, label: impl SystemLabel) {
        self.after.push(Box::new(label));
    }
    pub(crate) fn set_stage(&mut self, label: impl StageLabel) {
        self.stage = Some(Box::new(label));
    }
    pub(crate) fn add_ambiguity_set(&mut self, set: impl AmbiguitySetLabel) {
        self.ambiguity_sets.push(Box::new(set));
    }
    pub(crate) fn set_run_criteria(&mut self, criteria: RunCriteriaDescriptorOrLabel) {
        self.run_criteria = Some(criteria);
    }
    pub(crate) fn startup(&mut self) {
        self.startup = true;
    }
    pub(crate) fn set_insertion_point(&mut self, insertion_point: InsertionPoint) {
        self.insertion_point = insertion_point;
    }
}

pub struct ParallelSystemKind;
pub struct SystemSetKind;
