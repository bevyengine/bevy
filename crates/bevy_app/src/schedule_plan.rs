use crate::System;
use legion::prelude::Schedule;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

#[derive(Default)]
pub struct SchedulePlan {
    stages: HashMap<String, Vec<System>>,
    stage_order: Vec<String>,
    system_names: HashSet<Cow<'static, str>>,
}

impl SchedulePlan {
    pub fn build(&mut self) -> Schedule {
        let mut schedule_builder = Schedule::builder();

        for stage in self.stage_order.drain(..) {
            if let Some((_, mut systems)) = self.stages.remove_entry(&stage) {
                let system_count = systems.len();
                for system in systems.drain(..) {
                    match system {
                        System::Schedulable(schedulable) => {
                            schedule_builder = schedule_builder.add_system(schedulable);
                        }
                        System::ThreadLocal(runnable) => {
                            schedule_builder = schedule_builder.add_thread_local(runnable);
                        }
                        System::ThreadLocalFn((_name, thread_local)) => {
                            schedule_builder = schedule_builder.add_thread_local_fn(thread_local);
                        }
                    }
                }

                if system_count > 0 {
                    schedule_builder = schedule_builder.flush();
                }
            }
        }

        schedule_builder.build()
    }

    pub fn add_stage(&mut self, stage: &str) {
        if let Some(_) = self.stages.get(stage) {
            panic!("Stage already exists: {}", stage);
        } else {
            self.stages.insert(stage.to_string(), Vec::new());
            self.stage_order.push(stage.to_string());
        }
    }

    pub fn add_stage_after(&mut self, target: &str, stage: &str) {
        if let Some(_) = self.stages.get(stage) {
            panic!("Stage already exists: {}", stage);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage)| stage.as_str() == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}", target));

        self.stages.insert(stage.to_string(), Vec::new());
        self.stage_order.insert(target_index + 1, stage.to_string());
    }

    pub fn add_stage_before(&mut self, target: &str, stage: &str) {
        if let Some(_) = self.stages.get(stage) {
            panic!("Stage already exists: {}", stage);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage)| stage.as_str() == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}", target));

        self.stages.insert(stage.to_string(), Vec::new());
        self.stage_order.insert(target_index, stage.to_string());
    }

    pub fn add_system_to_stage(
        &mut self,
        stage_name: &str,
        system: impl Into<System>,
    ) -> &mut Self {
        let systems = self
            .stages
            .get_mut(stage_name)
            .unwrap_or_else(|| panic!("Stage does not exist: {}", stage_name));
        let system = system.into();
        let system_name = system.name();
        if self.system_names.contains(&system_name) {
            panic!("System with name {} already exists", system_name);
        }
        self.system_names.insert(system_name);
        systems.push(system.into());

        self
    }
}