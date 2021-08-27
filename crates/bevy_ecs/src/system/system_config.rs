use crate::schedule::BoxedStageLabel;
use crate::schedule::BoxedSystemLabel;
use crate::schedule::StageLabel;
use crate::schedule::SystemLabel;

use super::System;

#[derive(Default)]
pub struct SystemConfig {
    labels: Vec<BoxedSystemLabel>,
    before: Vec<BoxedSystemLabel>,
    after: Vec<BoxedSystemLabel>,
    stages: Vec<BoxedStageLabel>,
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
    pub(crate) fn add_to_stage(&mut self, label: impl StageLabel) {
        self.stages.push(Box::new(label));
    }
}

trait ScheduleSystem {
    fn label(&mut self, label: impl SystemLabel) -> &mut Self;
    fn before(&mut self, label: impl SystemLabel) -> &mut Self;
    fn after(&mut self, label: impl SystemLabel) -> &mut Self;
}

impl<T: System> ScheduleSystem for T {
    fn label(&mut self, label: impl SystemLabel) -> &mut Self {
        self.config().add_label(label);
        self
    }
    fn before(&mut self, label: impl SystemLabel) -> &mut Self {
        self.config().add_before(label);
        self
    }
    fn after(&mut self, label: impl SystemLabel) -> &mut Self {
        self.config().add_after(label);
        self
    }
}

pub trait StageSystem {
    fn add_to_stage(&mut self, label: impl StageLabel) -> &mut Self;
}

impl<T: System> StageSystem for T {
    fn add_to_stage(&mut self, label: impl StageLabel) -> &mut Self {
        self.config().add_to_stage(label);
        self
    }
}
