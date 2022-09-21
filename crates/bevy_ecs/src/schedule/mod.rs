//! Tools for controlling system execution.
//!
//! When using Bevy ECS, systems are usually not run directly, but are inserted into a
//!  [`Stage`], which then lives within a [`Schedule`].

mod ambiguity_detection;
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

use crate::{system::IntoSystem, world::World};
use bevy_utils::HashMap;

/// A container of [`Stage`]s set to be run in a linear order.
///
/// Since `Schedule` implements the [`Stage`] trait, it can be inserted into another schedule.
/// In this way, the properties of the child schedule can be set differently from the parent.
/// For example, it can be set to run only once during app execution, while the parent schedule
/// runs indefinitely.
#[derive(Default)]
pub struct Schedule {
    stages: HashMap<StageLabelId, Box<dyn Stage>>,
    stage_order: Vec<StageLabelId>,
    run_criteria: BoxedRunCriteria,
}

impl Schedule {
    /// Similar to [`add_stage`](Self::add_stage), but it also returns itself.
    #[must_use]
    pub fn with_stage<S: Stage>(mut self, label: impl StageLabel, stage: S) -> Self {
        self.add_stage(label, stage);
        self
    }

    /// Similar to [`add_stage_after`](Self::add_stage_after), but it also returns itself.
    #[must_use]
    pub fn with_stage_after<S: Stage>(
        mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> Self {
        self.add_stage_after(target, label, stage);
        self
    }

    /// Similar to [`add_stage_before`](Self::add_stage_before), but it also returns itself.
    #[must_use]
    pub fn with_stage_before<S: Stage>(
        mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> Self {
        self.add_stage_before(target, label, stage);
        self
    }

    #[must_use]
    pub fn with_run_criteria<S: IntoSystem<(), ShouldRun, P>, P>(mut self, system: S) -> Self {
        self.set_run_criteria(system);
        self
    }

    /// Similar to [`add_system_to_stage`](Self::add_system_to_stage), but it also returns itself.
    #[must_use]
    pub fn with_system_in_stage<Params>(
        mut self,
        stage_label: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> Self {
        self.add_system_to_stage(stage_label, system);
        self
    }

    pub fn set_run_criteria<S: IntoSystem<(), ShouldRun, P>, P>(&mut self, system: S) -> &mut Self {
        self.run_criteria
            .set(Box::new(IntoSystem::into_system(system)));
        self
    }

    /// Adds the given `stage` at the last position of the schedule.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut schedule = Schedule::default();
    /// // Define a new label for the stage.
    /// #[derive(StageLabel)]
    /// struct MyStage;
    /// // Add a stage with that label to the schedule.
    /// schedule.add_stage(MyStage, SystemStage::parallel());
    /// ```
    pub fn add_stage<S: Stage>(&mut self, label: impl StageLabel, stage: S) -> &mut Self {
        let label = label.as_label();
        self.stage_order.push(label);
        let prev = self.stages.insert(label, Box::new(stage));
        assert!(prev.is_none(), "Stage already exists: {:?}.", label);
        self
    }

    /// Adds the given `stage` immediately after the `target` stage.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut schedule = Schedule::default();
    /// # #[derive(StageLabel)]
    /// # struct TargetStage;
    /// # schedule.add_stage(TargetStage, SystemStage::parallel());
    /// // Define a new label for the stage.
    /// #[derive(StageLabel)]
    /// struct NewStage;
    /// // Add a stage with that label to the schedule.
    /// schedule.add_stage_after(TargetStage, NewStage, SystemStage::parallel());
    /// ```
    pub fn add_stage_after<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        let label = label.as_label();
        let target = target.as_label();
        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_label)| **stage_label == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {:?}.", target));

        self.stage_order.insert(target_index + 1, label);
        let prev = self.stages.insert(label, Box::new(stage));
        assert!(prev.is_none(), "Stage already exists: {:?}.", label);
        self
    }

    /// Adds the given `stage` immediately before the `target` stage.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut schedule = Schedule::default();
    /// # #[derive(StageLabel)]
    /// # struct TargetStage;
    /// # schedule.add_stage(TargetStage, SystemStage::parallel());
    /// #
    /// // Define a new, private label for the stage.
    /// #[derive(StageLabel)]
    /// struct NewStage;
    /// // Add a stage with that label to the schedule.
    /// schedule.add_stage_before(TargetStage, NewStage, SystemStage::parallel());
    /// ```
    pub fn add_stage_before<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        let label = label.as_label();
        let target = target.as_label();
        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_label)| **stage_label == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {:?}.", target));

        self.stage_order.insert(target_index, label);
        let prev = self.stages.insert(label, Box::new(stage));
        assert!(prev.is_none(), "Stage already exists: {:?}.", label);
        self
    }

    /// Adds the given `system` to the stage identified by `stage_label`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # fn my_system() {}
    /// # let mut schedule = Schedule::default();
    /// # #[derive(StageLabel)]
    /// # struct MyStage;
    /// # schedule.add_stage(MyStage, SystemStage::parallel());
    /// #
    /// schedule.add_system_to_stage(MyStage, my_system);
    /// ```
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

        let label = stage_label.as_label();
        let stage = self
            .get_stage_mut::<SystemStage>(label)
            .unwrap_or_else(move || stage_not_found(&label));
        stage.add_system(system);
        self
    }

    /// Adds the given `system_set` to the stage identified by `stage_label`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # fn my_system() {}
    /// # let mut schedule = Schedule::default();
    /// # #[derive(StageLabel)]
    /// # struct MyStage;
    /// # schedule.add_stage(MyStage, SystemStage::parallel());
    /// #
    /// schedule.add_system_set_to_stage(
    ///     MyStage,
    ///     SystemSet::new()
    ///         .with_system(system_a)
    ///         .with_system(system_b)
    ///         .with_system(system_c)
    /// );
    /// #
    /// # fn system_a() {}
    /// # fn system_b() {}
    /// # fn system_c() {}
    /// ```
    pub fn add_system_set_to_stage(
        &mut self,
        stage_label: impl StageLabel,
        system_set: SystemSet,
    ) -> &mut Self {
        self.stage(stage_label, |stage: &mut SystemStage| {
            stage.add_system_set(system_set)
        })
    }

    /// Fetches the [`Stage`] of type `T` marked with `label`, then executes the provided
    /// `func` passing the fetched stage to it as an argument.
    ///
    /// The `func` argument should be a function or a closure that accepts a mutable reference
    /// to a struct implementing `Stage` and returns the same type. That means that it should
    /// also assume that the stage has already been fetched successfully.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut schedule = Schedule::default();
    /// # #[derive(StageLabel)]
    /// # struct MyStage;
    /// # schedule.add_stage(MyStage, SystemStage::parallel());
    /// #
    /// schedule.stage(MyStage, |stage: &mut SystemStage| {
    ///     stage.add_system(my_system)
    /// });
    /// #
    /// # fn my_system() {}
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `label` refers to a non-existing stage, or if it's not of type `T`.
    pub fn stage<T: Stage, F: FnOnce(&mut T) -> &mut T>(
        &mut self,
        stage_label: impl StageLabel,
        func: F,
    ) -> &mut Self {
        let label = stage_label.as_label();
        let stage = self.get_stage_mut::<T>(label).unwrap_or_else(move || {
            panic!("stage '{label:?}' does not exist or is the wrong type",)
        });
        func(stage);
        self
    }

    /// Returns a shared reference to the stage identified by `label`, if it exists.
    ///
    /// If the requested stage does not exist, `None` is returned instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # fn my_system() {}
    /// # let mut schedule = Schedule::default();
    /// # #[derive(StageLabel)]
    /// # struct MyStage;
    /// # schedule.add_stage(MyStage, SystemStage::parallel());
    /// #
    /// let stage = schedule.get_stage::<SystemStage>(MyStage).unwrap();
    /// ```
    pub fn get_stage<T: Stage>(&self, stage_label: impl StageLabel) -> Option<&T> {
        let label = stage_label.as_label();
        self.stages
            .get(&label)
            .and_then(|stage| stage.downcast_ref::<T>())
    }

    /// Returns a unique, mutable reference to the stage identified by `label`, if it exists.
    ///
    /// If the requested stage does not exist, `None` is returned instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # fn my_system() {}
    /// # let mut schedule = Schedule::default();
    /// # #[derive(StageLabel)]
    /// # struct MyStage;
    /// # schedule.add_stage(MyStage, SystemStage::parallel());
    /// #
    /// let stage = schedule.get_stage_mut::<SystemStage>(MyStage).unwrap();
    /// ```
    pub fn get_stage_mut<T: Stage>(&mut self, stage_label: impl StageLabel) -> Option<&mut T> {
        let label = stage_label.as_label();
        self.stages
            .get_mut(&label)
            .and_then(|stage| stage.downcast_mut::<T>())
    }

    /// Executes each [`Stage`] contained in the schedule, one at a time.
    pub fn run_once(&mut self, world: &mut World) {
        for label in &self.stage_order {
            #[cfg(feature = "trace")]
            let _stage_span = bevy_utils::tracing::info_span!("stage", name = ?label).entered();
            let stage = self.stages.get_mut(label).unwrap();
            stage.run(world);
        }
    }

    /// Iterates over all of schedule's stages and their labels, in execution order.
    pub fn iter_stages(&self) -> impl Iterator<Item = (StageLabelId, &dyn Stage)> {
        self.stage_order
            .iter()
            .map(move |&label| (label, &*self.stages[&label]))
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
