mod executor;
mod executor_parallel;
pub mod graph_utils;
mod label;
mod run_criteria;
mod stage;
mod state;
mod system_container;
mod system_descriptor;
mod system_set;

pub use executor::*;
pub use executor_parallel::*;
pub use graph_utils::GraphNode;
pub use label::*;
pub use run_criteria::*;
pub use stage::*;
pub use state::*;
pub use system_container::*;
pub use system_descriptor::*;
pub use system_set::*;

use std::fmt::Debug;

use crate::{
    system::{IntoSystem, System},
    world::World,
};
use bevy_utils::HashMap;

#[derive(Default)]
pub struct Schedule {
    stages: HashMap<BoxedStageLabel, Box<dyn Stage>>,
    stage_order: Vec<BoxedStageLabel>,
    run_criteria: BoxedRunCriteria,
}

impl Schedule {
    pub fn with_stage<S: Stage>(mut self, label: impl StageLabel, stage: S) -> Self {
        self.add_stage(label, stage);
        self
    }

    pub fn with_stage_after<S: Stage>(
        mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> Self {
        self.add_stage_after(target, label, stage);
        self
    }

    pub fn with_stage_before<S: Stage>(
        mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> Self {
        self.add_stage_before(target, label, stage);
        self
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.set_run_criteria(system);
        self
    }

    pub fn with_system_in_stage<Params>(
        mut self,
        stage_label: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> Self {
        self.add_system_to_stage(stage_label, system);
        self
    }

    pub fn set_run_criteria<S: System<In = (), Out = ShouldRun>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.run_criteria.set(Box::new(system.system()));
        self
    }

    pub fn add_stage<S: Stage>(&mut self, label: impl StageLabel, stage: S) -> &mut Self {
        let label: Box<dyn StageLabel> = Box::new(label);
        self.stage_order.push(label.clone());
        let prev = self.stages.insert(label.clone(), Box::new(stage));
        if prev.is_some() {
            panic!("Stage already exists: {:?}.", label);
        }
        self
    }

    pub fn add_stage_after<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        let label: Box<dyn StageLabel> = Box::new(label);
        let target = &target as &dyn StageLabel;
        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_label)| &***stage_label == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {:?}.", target));

        self.stage_order.insert(target_index + 1, label.clone());
        let prev = self.stages.insert(label.clone(), Box::new(stage));
        if prev.is_some() {
            panic!("Stage already exists: {:?}.", label);
        }
        self
    }

    pub fn add_stage_before<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        let label: Box<dyn StageLabel> = Box::new(label);
        let target = &target as &dyn StageLabel;
        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_label)| &***stage_label == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {:?}.", target));

        self.stage_order.insert(target_index, label.clone());
        let prev = self.stages.insert(label.clone(), Box::new(stage));
        if prev.is_some() {
            panic!("Stage already exists: {:?}.", label);
        }
        self
    }

    pub fn add_system_to_stage<Params>(
        &mut self,
        stage_label: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        // Use a function instead of a closure to ensure that it is codegend inside bevy_ecs instead
        // of the game. Closures inherit generic parameters from their enclosing function.
        #[cold]
        fn stage_not_found(stage_label: &dyn Debug) -> ! {
            panic!(
                "Stage '{:?}' does not exist or is not a SystemStage",
                stage_label
            )
        }

        let stage = self
            .get_stage_mut::<SystemStage>(&stage_label)
            .unwrap_or_else(move || stage_not_found(&stage_label));
        stage.add_system(system);
        self
    }

    pub fn add_system_set_to_stage(
        &mut self,
        stage_label: impl StageLabel,
        system_set: SystemSet,
    ) -> &mut Self {
        self.stage(stage_label, |stage: &mut SystemStage| {
            stage.add_system_set(system_set)
        })
    }

    pub fn stage<T: Stage, F: FnOnce(&mut T) -> &mut T>(
        &mut self,
        label: impl StageLabel,
        func: F,
    ) -> &mut Self {
        let stage = self.get_stage_mut::<T>(&label).unwrap_or_else(move || {
            panic!("stage '{:?}' does not exist or is the wrong type", label)
        });
        func(stage);
        self
    }

    pub fn get_stage<T: Stage>(&self, label: &dyn StageLabel) -> Option<&T> {
        self.stages
            .get(label)
            .and_then(|stage| stage.downcast_ref::<T>())
    }

    pub fn get_stage_mut<T: Stage>(&mut self, label: &dyn StageLabel) -> Option<&mut T> {
        self.stages
            .get_mut(label)
            .and_then(|stage| stage.downcast_mut::<T>())
    }

    pub fn run_once(&mut self, world: &mut World) {
        for label in self.stage_order.iter() {
            #[cfg(feature = "trace")]
            let stage_span =
                bevy_utils::tracing::info_span!("stage", name = &format!("{:?}", label) as &str);
            #[cfg(feature = "trace")]
            let _stage_guard = stage_span.enter();
            let stage = self.stages.get_mut(label).unwrap();
            stage.run(world);
        }
    }

    /// Iterates over all of schedule's stages and their labels, in execution order.
    pub fn iter_stages(&self) -> impl Iterator<Item = (&dyn StageLabel, &dyn Stage)> {
        self.stage_order
            .iter()
            .map(move |label| (&**label, &*self.stages[label]))
    }
}

impl Stage for Schedule {
    fn run(&mut self, world: &mut World) {
        loop {
            match self.run_criteria.should_run(world) {
                ShouldRun::No => return,
                ShouldRun::Yes => {
                    self.run_once(world);
                    return;
                }
                ShouldRun::YesAndCheckAgain => {
                    self.run_once(world);
                }
                ShouldRun::NoAndCheckAgain => {
                    panic!("`NoAndCheckAgain` would loop infinitely in this situation.")
                }
            }
        }
    }
}
