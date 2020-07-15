use crate::{system::{ArchetypeAccess, ThreadLocalExecution}, Resources, Schedule, System};
use crossbeam_channel::{Receiver, Sender};
use fixedbitset::FixedBitSet;
use hecs::{World};
use rayon::ScopeFifo;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct ParallelExecutor {
    stages: Vec<ExecutorStage>,
}

impl ParallelExecutor {
    pub fn prepare(&mut self, schedule: &mut Schedule, world: &World) {
        // TODO: if world archetype generation hasnt changed, dont update here

        let mut executor_stages = vec![ExecutorStage::default(); schedule.stage_order.len()];
        for (stage_index, stage_name) in schedule.stage_order.iter().enumerate() {
            let executor_stage = &mut executor_stages[stage_index];
            // ensure finished dependencies has the required number of bits
            if let Some(systems) = schedule.stages.get(stage_name) {
                executor_stage.prepare(world, systems);
            }
        }

        self.stages = executor_stages;
    }

    pub fn run(&mut self, schedule: &mut Schedule, world: &mut World, resources: &mut Resources) {
        self.prepare(schedule, world);
        for (stage_name, executor_stage) in schedule.stage_order.iter().zip(self.stages.iter_mut())
        {
            if let Some(stage_systems) = schedule.stages.get_mut(stage_name) {
                executor_stage.run(world, resources, stage_systems);
            }
        }
    }
}

#[derive(Clone)]
pub struct ExecutorStage {
    /// each system's set of dependencies
    system_dependencies: Vec<FixedBitSet>,
    /// each system's dependents (the systems that can't run until this system has run)
    system_dependents: Vec<Vec<usize>>,

    /// the currently finished systems
    finished_systems: FixedBitSet,
    running_systems: FixedBitSet,

    sender: Sender<usize>,
    receiver: Receiver<usize>,
}

impl Default for ExecutorStage {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            system_dependents: Default::default(),
            system_dependencies: Default::default(),
            finished_systems: Default::default(),
            running_systems: Default::default(),
            sender,
            receiver,
        }
    }
}

enum RunReadyResult {
    Ok,
    ThreadLocalReady(usize),
}

enum RunReadyType {
    All,
    Dependents(usize),
}

impl ExecutorStage {
    pub fn prepare(&mut self, world: &World, systems: &Vec<Arc<Mutex<Box<dyn System>>>>) {
        self.system_dependencies = vec![FixedBitSet::with_capacity(systems.len()); systems.len()];
        self.system_dependents = vec![Vec::new(); systems.len()];
        self.finished_systems.grow(systems.len());
        self.running_systems.grow(systems.len());

        // update each system's archetype access to latest world archetypes
        for system in systems.iter() {
            let mut system = system.lock().unwrap();
            system.update_archetype_access(world);
        }

        let mut current_archetype_access = ArchetypeAccess::default();
        let mut last_thread_local_index: Option<usize> = None;
        for (system_index, system) in systems.iter().enumerate() {
            let system = system.lock().unwrap();
            let archetype_access = system.get_archetype_access();
            match system.thread_local_execution() {
                ThreadLocalExecution::NextFlush => {
                    // if any system before this one conflicts, check all systems that came before for compatibility
                    if current_archetype_access.is_compatible(archetype_access) == false {
                        for earlier_system_index in 0..system_index {
                            let earlier_system = systems[earlier_system_index].lock().unwrap();

                            // ignore "immediate" thread local systems, we handle them separately
                            if let ThreadLocalExecution::Immediate =
                                earlier_system.thread_local_execution()
                            {
                                continue;
                            }

                            let earlier_archetype_access = earlier_system.get_archetype_access();
                            // if earlier system is incompatible, make the current system dependent
                            if earlier_archetype_access.is_compatible(archetype_access) == false {
                                self.system_dependents[earlier_system_index].push(system_index);
                                self.system_dependencies[system_index].insert(earlier_system_index);
                            }
                        }
                    }

                    current_archetype_access.union(archetype_access);

                    if let Some(last_thread_local_index) = last_thread_local_index {
                        self.system_dependents[last_thread_local_index].push(system_index);
                        self.system_dependencies[system_index].insert(last_thread_local_index);
                    }
                }
                ThreadLocalExecution::Immediate => {
                    last_thread_local_index = Some(system_index);
                    for earlier_system_index in 0..system_index {
                        // treat all earlier systems as "incompatible" to ensure we run this thread local system exclusively
                        self.system_dependents[earlier_system_index].push(system_index);
                        self.system_dependencies[system_index].insert(earlier_system_index);
                    }
                }
            }
        }
    }

    fn run_ready_systems<'run>(
        &mut self,
        systems: &[Arc<Mutex<Box<dyn System>>>],
        run_ready_type: RunReadyType,
        scope: &ScopeFifo<'run>,
        world: &'run World,
        resources: &'run Resources,
    ) -> RunReadyResult {
        // produce a system index iterator based on the passed in RunReadyType
        let mut all;
        let mut dependents;
        let system_index_iter: &mut dyn Iterator<Item = usize> = match run_ready_type {
            RunReadyType::All => {
                all = 0..systems.len();
                &mut all
            }
            RunReadyType::Dependents(system_index) => {
                dependents = self.system_dependents[system_index].iter().cloned();
                &mut dependents
            }
        };

        let mut systems_currently_running = false;
        for system_index in system_index_iter {
            // if this system has already been run, don't try to run it again
            if self.running_systems.contains(system_index) {
                continue;
            }

            // if all system dependencies are finished, queue up the system to run
            if self.system_dependencies[system_index].is_subset(&self.finished_systems) {
                let system = systems[system_index].clone();

                // handle thread local system
                {
                    let system = system.lock().unwrap();
                    if let ThreadLocalExecution::Immediate = system.thread_local_execution() {
                        if systems_currently_running {
                            // if systems are currently running, we can't run this thread local system yet
                            continue;
                        } else {
                            // if no systems are running, return this thread local system to be run exclusively
                            return RunReadyResult::ThreadLocalReady(system_index);
                        }
                    }
                }

                // handle multi-threaded system
                let sender = self.sender.clone();
                self.running_systems.insert(system_index);
                scope.spawn_fifo(move |_| {
                    let mut system = system.lock().unwrap();
                    system.run(world, resources);
                    sender.send(system_index).unwrap();
                });

                systems_currently_running = true;
            }
        }

        RunReadyResult::Ok
    }

    pub fn run(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        systems: &[Arc<Mutex<Box<dyn System>>>],
    ) {
        self.finished_systems.clear();
        self.running_systems.clear();
        let mut run_ready_result = RunReadyResult::Ok;
        rayon::scope_fifo(|scope| {
            run_ready_result =
                self.run_ready_systems(systems, RunReadyType::All, scope, world, resources);
        });
        loop {
            // if all systems in the stage are finished, break out of the loop
            if self.finished_systems.count_ones(..) == systems.len() {
                break;
            }

            if let RunReadyResult::ThreadLocalReady(thread_local_index) = run_ready_result {
                // if a thread local system is ready to run, run it exclusively on the main thread
                let mut system = systems[thread_local_index].lock().unwrap();
                self.running_systems.insert(thread_local_index);
                system.run(world, resources);
                system.run_thread_local(world, resources);
                self.finished_systems.insert(thread_local_index);
                self.sender.send(thread_local_index).unwrap();

                // TODO: if archetype generation has changed, call "prepare" on all systems after this one
                
                run_ready_result = RunReadyResult::Ok;
            } else {
                // wait for a system to finish, then run its dependents
                rayon::scope_fifo(|scope| {
                    loop {
                        // if all systems in the stage are finished, break out of the loop
                        if self.finished_systems.count_ones(..) == systems.len() {
                            break;
                        }

                        let finished_system = self.receiver.recv().unwrap();
                        self.finished_systems.insert(finished_system);
                        run_ready_result = self.run_ready_systems(
                            systems,
                            RunReadyType::Dependents(finished_system),
                            scope,
                            world,
                            resources,
                        );

                        // if the next ready system is thread local, break out of this loop/rayon scope so it can be run
                        if let RunReadyResult::ThreadLocalReady(_) = run_ready_result {
                            break;
                        }
                    }
                });
            }
        }

        // "flush"
        for system in systems.iter() {
            let mut system = system.lock().unwrap();
            match system.thread_local_execution() {
                ThreadLocalExecution::NextFlush => system.run_thread_local(world, resources),
                ThreadLocalExecution::Immediate => { /* already ran */ }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParallelExecutor;
    use crate::{IntoQuerySystem, IntoThreadLocalSystem, Query, Res, Resources, Schedule, World};
    use fixedbitset::FixedBitSet;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Counter {
        count: Arc<Mutex<usize>>,
    }

    fn read_u32(counter: Res<Counter>, _query: Query<&u32>) {
        let mut count = counter.count.lock().unwrap();
        assert!(
            *count < 2,
            "read_32 should be one of the first two systems to run"
        );
        *count += 1;
    }

    fn write_float(counter: Res<Counter>, _query: Query<&f32>) {
        let mut count = counter.count.lock().unwrap();
        assert!(
            *count < 2,
            "write_float should be one of the first two systems to run"
        );
        *count += 1;
    }

    fn read_u32_write_u64(counter: Res<Counter>, _query: Query<(&u32, &mut u64)>) {
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 2,
            "read_u32_write_u64 should always be the third system to run"
        );
        *count += 1;
    }

    fn read_u64(counter: Res<Counter>, _query: Query<&u64>) {
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 3,
            "read_u64 should always be the fourth system to run"
        );
        *count += 1;
    }

    fn write_u64(counter: Res<Counter>, _query: Query<&mut u64>) {
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 4,
            "write_u64 should always be the fifth system to run"
        );
        *count += 1;
    }

    fn thread_local_system(_world: &mut World, resources: &mut Resources) {
        let counter = resources.get::<Counter>().unwrap();
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 5,
            "thread_local_system should always be the sixth system to run"
        );
        *count += 1;
    }

    fn write_f32(counter: Res<Counter>, _query: Query<&mut f32>) {
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 6,
            "write_f32 should always be the seventh system to run"
        );
        *count += 1;
    }

    #[test]
    fn schedule() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Counter::default());

        world.spawn((1.0f32,));
        world.spawn((1u32, 1u64));
        world.spawn((2u32,));

        let mut schedule = Schedule::default();
        schedule.add_stage("A");
        schedule.add_stage("B");

        schedule.add_system_to_stage("A", read_u32.system());
        schedule.add_system_to_stage("A", write_float.system());
        schedule.add_system_to_stage("A", read_u32_write_u64.system());
        schedule.add_system_to_stage("A", read_u64.system());
        schedule.add_system_to_stage("B", write_u64.system());
        schedule.add_system_to_stage("B", thread_local_system.thread_local_system());
        schedule.add_system_to_stage("B", write_f32.system());

        let mut executor = ParallelExecutor::default();
        run_executor_and_validate(&mut executor, &mut schedule, &mut world, &mut resources);
        // run again (with counter reset) to ensure executor works correctly across runs
        *resources.get::<Counter>().unwrap().count.lock().unwrap() = 0;
        run_executor_and_validate(&mut executor, &mut schedule, &mut world, &mut resources);
    }

    fn run_executor_and_validate(executor: &mut ParallelExecutor, schedule: &mut Schedule, world: &mut World, resources: &mut Resources) {
        executor.prepare(schedule, world);

        assert_eq!(
            executor.stages[0].system_dependents,
            vec![vec![2], vec![], vec![3], vec![]]
        );
        assert_eq!(
            executor.stages[1].system_dependents,
            vec![vec![1], vec![2], vec![]]
        );

        let stage_0_len = executor.stages[0].system_dependencies.len();
        let mut read_u32_write_u64_deps = FixedBitSet::with_capacity(stage_0_len);
        read_u32_write_u64_deps.insert(0);
        let mut read_u64_deps = FixedBitSet::with_capacity(stage_0_len);
        read_u64_deps.insert(2);

        assert_eq!(
            executor.stages[0].system_dependencies,
            vec![
                FixedBitSet::with_capacity(stage_0_len),
                FixedBitSet::with_capacity(stage_0_len),
                read_u32_write_u64_deps,
                read_u64_deps,
            ]
        );

        let stage_1_len = executor.stages[1].system_dependencies.len();
        let mut thread_local_deps = FixedBitSet::with_capacity(stage_1_len);
        thread_local_deps.insert(0);
        let mut write_f64_deps = FixedBitSet::with_capacity(stage_1_len);
        write_f64_deps.insert(1);
        assert_eq!(
            executor.stages[1].system_dependencies,
            vec![
                FixedBitSet::with_capacity(stage_1_len),
                thread_local_deps,
                write_f64_deps
            ]
        );

        executor.run(schedule, world, resources);

        let counter = resources.get::<Counter>().unwrap();
        assert_eq!(
            *counter.count.lock().unwrap(),
            7,
            "counter should have been incremented once for each system"
        );
    }
}
