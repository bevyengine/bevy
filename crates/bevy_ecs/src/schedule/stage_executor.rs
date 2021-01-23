use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};

use crate::{
    ExclusiveSystem, Resources,
    ShouldRun::{self, *},
    SystemIndex, SystemSet, World,
};

pub trait SystemStageExecutor: Downcast + Send + Sync {
    /// Runs the systems in the given system sets, using the ordering rules provided by arguments.
    ///
    /// * `system_sets`: Groups of systems; each set has its own run criterion.
    /// * `at_start`: Topologically sorted exclusive systems that want to
    /// be ran at the start of the stage.
    /// * `before_commands`: Topologically sorted exclusive systems that want to
    /// be ran after parallel systems but before the application of their command buffers.
    /// * `at_end`: Topologically sorted exclusive systems that want to
    /// be ran at the end of the stage.
    /// * `parallel_dependencies`: Resolved graph of parallel systems and their dependencies.
    /// Contains all parallel systems.
    /// * `parallel_sorted`: Topologically sorted parallel systems.
    #[allow(clippy::too_many_arguments)] // Hmm...
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        at_start: &[SystemIndex],
        before_commands: &[SystemIndex],
        at_end: &[SystemIndex],
        parallel_dependencies: &HashMap<SystemIndex, Vec<SystemIndex>>,
        parallel_sorted: &[SystemIndex],
        world: &mut World,
        resources: &mut Resources,
    );
}

impl_downcast!(SystemStageExecutor);

pub(super) trait ExecutorCommonMethods {
    fn system_set_should_run(&self) -> &Vec<ShouldRun>;
    fn system_set_should_run_mut(&mut self) -> &mut Vec<ShouldRun>;

    /// Populates `system_set_should_run` with results of system sets' run criteria evaluation,
    /// returns true if any of the sets should be ran.
    /// # Panics
    /// Panics if all criteria evaluate to a combination of `No` and at least one `NoAndLoop`.
    fn evaluate_run_criteria(
        &mut self,
        system_sets: &mut [SystemSet],
        world: &mut World,
        resources: &mut Resources,
    ) -> bool {
        let mut has_any_work = false;
        let mut has_doable_work = false;
        self.system_set_should_run_mut().clear();
        self.system_set_should_run_mut()
            .extend(system_sets.iter_mut().map(|set| {
                let result = set.run_criteria_mut().should_run(world, resources);
                match result {
                    Yes | YesAndLoop => {
                        has_doable_work = true;
                        has_any_work = true;
                    }
                    NoAndLoop => has_any_work = true,
                    No => (),
                }
                result
            }));
        // TODO a real error message
        assert!(!has_any_work || has_doable_work);
        has_doable_work
    }

    /// Updates `system_set_should_run` under assumption that an iteration has been completed,
    /// returns true if any of the sets should be ran.
    /// # Panics
    /// Panics if all criteria evaluate to a combination of `No` and at least one `NoAndLoop`.
    fn reevaluate_run_criteria(
        &mut self,
        system_sets: &mut [SystemSet],
        world: &mut World,
        resources: &mut Resources,
    ) -> bool {
        let mut has_any_work = false;
        let mut has_doable_work = false;
        for (index, result) in self.system_set_should_run_mut().iter_mut().enumerate() {
            match result {
                No => (),
                Yes => *result = No,
                YesAndLoop | NoAndLoop => {
                    let new_result = system_sets[index]
                        .run_criteria_mut()
                        .should_run(world, resources);
                    match new_result {
                        Yes | YesAndLoop => {
                            has_doable_work = true;
                            has_any_work = true;
                        }
                        NoAndLoop => has_any_work = true,
                        No => (),
                    }
                    *result = new_result;
                }
            }
        }
        // TODO a real error message
        assert!(!has_any_work || has_doable_work);
        has_doable_work
    }

    fn run_exclusive_systems_sequence(
        &self,
        sequence: &[SystemIndex],
        system_sets: &mut [SystemSet],
        world: &mut World,
        resources: &mut Resources,
    ) {
        for index in sequence {
            if let Yes | YesAndLoop = self.system_set_should_run()[index.set] {
                let system = system_sets[index.set].exclusive_system_mut(index.system);
                system.run(world, resources);
            }
        }
    }
}

pub struct SingleThreadedSystemStageExecutor {
    /// Cached results of system sets' run criteria evaluation.
    system_set_should_run: Vec<ShouldRun>,
}

impl Default for SingleThreadedSystemStageExecutor {
    fn default() -> Self {
        Self {
            system_set_should_run: Vec::new(),
        }
    }
}

impl ExecutorCommonMethods for SingleThreadedSystemStageExecutor {
    fn system_set_should_run(&self) -> &Vec<ShouldRun> {
        &self.system_set_should_run
    }

    fn system_set_should_run_mut(&mut self) -> &mut Vec<ShouldRun> {
        &mut self.system_set_should_run
    }
}

impl SystemStageExecutor for SingleThreadedSystemStageExecutor {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        at_start: &[SystemIndex],
        before_commands: &[SystemIndex],
        at_end: &[SystemIndex],
        _parallel_dependencies: &HashMap<SystemIndex, Vec<SystemIndex>>,
        parallel_sorted: &[SystemIndex],
        world: &mut World,
        resources: &mut Resources,
    ) {
        let mut has_work = self.evaluate_run_criteria(system_sets, world, resources);
        while has_work {
            // Run systems that want to be at the start of stage.
            self.run_exclusive_systems_sequence(at_start, system_sets, world, resources);

            // Run parallel systems.
            for index in parallel_sorted {
                if let Yes | YesAndLoop = self.system_set_should_run()[index.set] {
                    let system = system_sets[index.set].parallel_system_mut(index.system);
                    system.run((), world, resources);
                }
            }

            // Run systems that want to be between parallel systems and their command buffers.
            self.run_exclusive_systems_sequence(before_commands, system_sets, world, resources);

            // Apply parallel systems' buffers.
            for index in parallel_sorted {
                if let Yes | YesAndLoop = self.system_set_should_run()[index.set] {
                    let system = system_sets[index.set].parallel_system_mut(index.system);
                    system.apply_buffers(world, resources);
                }
            }

            // Run systems that want to be at the end of stage.
            self.run_exclusive_systems_sequence(at_end, system_sets, world, resources);

            has_work = self.reevaluate_run_criteria(system_sets, world, resources);
        }
    }
}
