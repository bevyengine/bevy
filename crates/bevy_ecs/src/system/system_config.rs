use crate::schedule::*;

use super::{ExclusiveSystem, InsertionPoint, System};

#[derive(Default)]
pub struct SystemConfig {
    pub labels: Vec<BoxedSystemLabel>,
    pub before: Vec<BoxedSystemLabel>,
    pub after: Vec<BoxedSystemLabel>,
    pub stage: Option<BoxedStageLabel>,
    pub ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
    pub run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub insertion_point: Option<InsertionPoint>,
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
}

pub struct ParallelSystemKind;
pub struct ExclusiveSystemKind;
pub struct SystemSetKind;

pub trait ScheduleConfig<SystemKind> {
    fn label(self, label: impl SystemLabel) -> Self;
    fn before(self, label: impl SystemLabel) -> Self;
    fn after(self, label: impl SystemLabel) -> Self;
}

impl<T: System> ScheduleConfig<ParallelSystemKind> for T {
    fn label(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_label(label);
        self
    }
    fn before(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_before(label);
        self
    }
    fn after(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_after(label);
        self
    }
}

impl ScheduleConfig<SystemSetKind> for SystemSet {
    fn label(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_label(label);
        self
    }
    fn before(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_before(label);
        self
    }
    fn after(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_after(label);
        self
    }
}

impl<T: ExclusiveSystem> ScheduleConfig<ExclusiveSystemKind> for T {
    fn label(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_label(label);
        self
    }
    fn before(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_before(label);
        self
    }
    fn after(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_after(label);
        self
    }
}

pub trait StageConfig<SystemKind> {
    fn stage(self, label: impl StageLabel) -> Self;
}

impl<T: System> StageConfig<ParallelSystemKind> for T {
    fn stage(mut self, label: impl StageLabel) -> Self {
        self.config_mut().set_stage(label);
        self
    }
}

impl StageConfig<SystemSetKind> for SystemSet {
    fn stage(mut self, label: impl StageLabel) -> Self {
        self.config_mut().set_stage(label);
        self
    }
}

impl<T: ExclusiveSystem> StageConfig<ExclusiveSystemKind> for T {
    fn stage(mut self, label: impl StageLabel) -> Self {
        self.config_mut().set_stage(label);
        self
    }
}

pub trait AmbiguityConfig<SystemKind> {
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> Self;
}

impl<T: System> AmbiguityConfig<ParallelSystemKind> for T {
    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.config_mut().add_ambiguity_set(set);
        self
    }
}

impl AmbiguityConfig<SystemSetKind> for SystemSet {
    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.config_mut().add_ambiguity_set(set);
        self
    }
}

impl<T: ExclusiveSystem> AmbiguityConfig<ExclusiveSystemKind> for T {
    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.config_mut().add_ambiguity_set(set);
        self
    }
}

pub trait RunCriteraConfig<SystemKind> {
    fn with_run_criteria<Marker>(self, run_criteria: impl IntoRunCriteria<Marker>) -> Self;
}

impl<T: System> RunCriteraConfig<ParallelSystemKind> for T {
    fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.config_mut().run_criteria = Some(run_criteria.into());
        self
    }
}

impl RunCriteraConfig<SystemSetKind> for SystemSet {
    fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.config_mut().run_criteria = Some(run_criteria.into());
        self
    }
}

impl<T: ExclusiveSystem> RunCriteraConfig<ExclusiveSystemKind> for T {
    fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.config_mut().run_criteria = Some(run_criteria.into());
        self
    }
}

pub trait ExclusiveConfig<SystemKind> {
    fn at_start(self) -> Self;
    fn before_commands(self) -> Self;
    fn at_end(self) -> Self;
}

impl<T: ExclusiveSystem> ExclusiveConfig<ExclusiveSystemKind> for T {
    fn at_start(mut self) -> Self {
        self.config_mut().insertion_point = Some(InsertionPoint::AtStart);
        self
    }
    fn before_commands(mut self) -> Self {
        self.config_mut().insertion_point = Some(InsertionPoint::BeforeCommands);
        self
    }
    fn at_end(mut self) -> Self {
        self.config_mut().insertion_point = Some(InsertionPoint::AtEnd);
        self
    }
}
