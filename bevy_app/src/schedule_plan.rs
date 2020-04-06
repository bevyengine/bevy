use legion::prelude::*;
use std::{cmp::Ordering, collections::HashMap};

enum System {
    Schedulable(Box<dyn Schedulable>),
    ThreadLocal(Box<dyn Runnable>),
    ThreadLocalFn(Box<dyn FnMut(&mut World, &mut Resources)>),
}

#[derive(Default)]
pub struct SchedulePlan {
    stages: HashMap<String, Vec<System>>,
    stage_order: Vec<String>,
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
                        System::ThreadLocalFn(thread_local) => {
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
        system: Box<dyn Schedulable>,
    ) -> &mut Self {
        let system = System::Schedulable(system);
        let systems = self
            .stages
            .get_mut(stage_name)
            .unwrap_or_else(|| panic!("Stage does not exist: {}", stage_name));
        systems.push(system);

        self
    }

    pub fn add_thread_local_to_stage(
        &mut self,
        stage_name: &str,
        runnable: Box<dyn Runnable>,
    ) -> &mut Self {
        let system = System::ThreadLocal(runnable);
        let systems = self
            .stages
            .get_mut(stage_name)
            .unwrap_or_else(|| panic!("Stage does not exist: {}", stage_name));
        systems.push(system);

        self
    }

    pub fn add_thread_local_fn_to_stage(
        &mut self,
        stage_name: &str,
        f: impl FnMut(&mut World, &mut Resources) + 'static,
    ) -> &mut Self {
        let system = System::ThreadLocalFn(Box::new(f));
        let systems = self
            .stages
            .get_mut(stage_name)
            .unwrap_or_else(|| panic!("Stage does not exist: {}", stage_name));
        systems.push(system);

        self
    }
}

// working around the famous "rust float ordering" problem
#[derive(PartialOrd)]
struct FloatOrd(f32);

impl Ord for FloatOrd {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or_else(|| {
            if self.0.is_nan() && !other.0.is_nan() {
                Ordering::Less
            } else if !self.0.is_nan() && other.0.is_nan() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
    }
}

impl PartialEq for FloatOrd {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            true
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for FloatOrd {}
