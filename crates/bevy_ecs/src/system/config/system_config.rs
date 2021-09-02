use crate::{schedule::*, system::InsertionPoint};

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
    pub fn add_label(&mut self, label: impl SystemLabel) {
        self.labels.push(Box::new(label));
    }
    pub fn add_before(&mut self, label: impl SystemLabel) {
        self.before.push(Box::new(label));
    }
    pub fn add_after(&mut self, label: impl SystemLabel) {
        self.after.push(Box::new(label));
    }
    pub fn set_stage(&mut self, label: impl StageLabel) {
        self.stage = Some(Box::new(label));
    }
    pub fn add_ambiguity_set(&mut self, set: impl AmbiguitySetLabel) {
        self.ambiguity_sets.push(Box::new(set));
    }
    pub fn set_run_criteria(&mut self, criteria: RunCriteriaDescriptorOrLabel) {
        self.run_criteria = Some(criteria);
    }
    pub fn startup(&mut self) {
        self.startup = true;
    }
    pub fn set_insertion_point(&mut self, insertion_point: InsertionPoint) {
        self.insertion_point = insertion_point;
    }
}

pub struct ParallelSystemKind;
pub struct SystemSetKind;
