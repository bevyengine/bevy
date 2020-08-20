use super::Schedule;
use crate::{
    resource::Resources,
    system::{ArchetypeAccess, System, ThreadLocalExecution, TypeAccess},
};
use bevy_hecs::{ArchetypesGeneration, World};
use crossbeam_channel::{Receiver, Sender};
use fixedbitset::FixedBitSet;
use rayon::ScopeFifo;
use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

/// Executes each schedule stage in parallel by analyzing system dependencies.
/// System execution order is undefined except under the following conditions:
/// * systems in earlier stages run before systems in later stages
/// * in a given stage, systems that mutate archetype X cannot run before systems registered before them that read/write archetype X
/// * in a given stage, systems the read archetype X cannot run before systems registered before them that write archetype X
/// * in a given stage, systems that mutate resource Y cannot run before systems registered before them that read/write resource Y
/// * in a given stage, systems the read resource Y cannot run before systems registered before them that write resource Y

#[derive(Debug)]
pub struct ParallelExecutor {
    stages: Vec<ExecutorStage>,
    last_schedule_generation: usize,
    clear_trackers: bool,
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self {
            stages: Default::default(),
            last_schedule_generation: usize::MAX, // MAX forces prepare to run the first time
            clear_trackers: true,
        }
    }
}

impl ParallelExecutor {
    pub fn without_tracker_clears() -> Self {
        Self {
            clear_trackers: false,
            ..Default::default()
        }
    }

    pub fn run(&mut self, schedule: &mut Schedule, world: &mut World, resources: &mut Resources) {
        let schedule_generation = schedule.generation();
        let schedule_changed = schedule.generation() != self.last_schedule_generation;
        if schedule_changed {
            self.stages.clear();
            self.stages
                .resize_with(schedule.stage_order.len(), ExecutorStage::default);
        }
        for (stage_name, executor_stage) in schedule.stage_order.iter().zip(self.stages.iter_mut())
        {
            if let Some(stage_systems) = schedule.stages.get_mut(stage_name) {
                executor_stage.run(world, resources, stage_systems, schedule_changed);
            }
        }

        if self.clear_trackers {
            world.clear_trackers();
        }

        self.last_schedule_generation = schedule_generation;
    }
}

/// This can be added as an app resource to control the global `rayon::ThreadPool` used by ecs.
// Dev internal note: We cannot directly expose a ThreadPoolBuilder here as it does not implement Send and Sync.
#[derive(Debug, Default, Clone)]
pub struct ParallelExecutorOptions {
    /// If some value, we'll set up the thread pool to use at most n threads. See `rayon::ThreadPoolBuilder::num_threads`.
    num_threads: Option<usize>,
    /// If some value, we'll set up the thread pool's' workers to the given stack size. See `rayon::ThreadPoolBuilder::stack_size`.
    stack_size: Option<usize>,
    // TODO: Do we also need/want to expose other features (*_handler, etc.)
}

impl ParallelExecutorOptions {
    /// Creates a new ParallelExecutorOptions instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the num_threads option, using the builder pattern
    pub fn with_num_threads(mut self, num_threads: Option<usize>) -> Self {
        self.num_threads = num_threads;
        self
    }

    /// Sets the stack_size option, using the builder pattern. WARNING: Only use this if you know what you're doing,
    /// otherwise your application may run into stability and performance issues.
    pub fn with_stack_size(mut self, stack_size: Option<usize>) -> Self {
        self.stack_size = stack_size;
        self
    }

    /// Creates a new ThreadPoolBuilder based on the current options.
    pub(crate) fn create_builder(&self) -> rayon::ThreadPoolBuilder {
        let mut builder = rayon::ThreadPoolBuilder::new();

        if let Some(num_threads) = self.num_threads {
            builder = builder.num_threads(num_threads);
        }

        if let Some(stack_size) = self.stack_size {
            builder = builder.stack_size(stack_size);
        }

        builder
    }
}

#[derive(Debug, Clone)]
pub struct ExecutorStage {
    /// each system's set of dependencies
    system_dependencies: Vec<FixedBitSet>,
    /// each system's dependents (the systems that can't run until this system has run)
    system_dependents: Vec<Vec<usize>>,
    /// stores the indices of thread local systems in this stage, which are used during stage.prepare()
    thread_local_system_indices: Vec<usize>,
    next_thread_local_index: usize,
    /// the currently finished systems
    finished_systems: FixedBitSet,
    running_systems: FixedBitSet,

    sender: Sender<usize>,
    receiver: Receiver<usize>,
    last_archetypes_generation: ArchetypesGeneration,
}

impl Default for ExecutorStage {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            system_dependents: Default::default(),
            system_dependencies: Default::default(),
            thread_local_system_indices: Default::default(),
            next_thread_local_index: 0,
            finished_systems: Default::default(),
            running_systems: Default::default(),
            sender,
            receiver,
            last_archetypes_generation: ArchetypesGeneration(u64::MAX), // MAX forces prepare to run the first time
        }
    }
}

enum RunReadyResult {
    Ok,
    ThreadLocalReady(usize),
}

enum RunReadyType {
    Range(Range<usize>),
    Dependents(usize),
}

impl ExecutorStage {
    pub fn prepare_to_next_thread_local(
        &mut self,
        world: &World,
        systems: &[Arc<Mutex<Box<dyn System>>>],
        schedule_changed: bool,
    ) {
        let (prepare_system_start_index, last_thread_local_index) =
            if self.next_thread_local_index == 0 {
                (0, None)
            } else {
                // start right after the last thread local system
                (
                    self.thread_local_system_indices[self.next_thread_local_index - 1] + 1,
                    Some(self.thread_local_system_indices[self.next_thread_local_index - 1]),
                )
            };

        let prepare_system_index_range = if let Some(index) = self
            .thread_local_system_indices
            .get(self.next_thread_local_index)
        {
            // if there is an upcoming thread local system, prepare up to (and including) it
            prepare_system_start_index..(*index + 1)
        } else {
            // if there are no upcoming thread local systems, prepare everything right now
            prepare_system_start_index..systems.len()
        };

        let archetypes_generation_changed =
            self.last_archetypes_generation != world.archetypes_generation();

        if schedule_changed || archetypes_generation_changed {
            // update each system's archetype access to latest world archetypes
            for system_index in prepare_system_index_range.clone() {
                let mut system = systems[system_index].lock().unwrap();
                system.update_archetype_access(world);
            }

            // calculate dependencies between systems and build execution order
            let mut current_archetype_access = ArchetypeAccess::default();
            let mut current_resource_access = TypeAccess::default();
            for system_index in prepare_system_index_range.clone() {
                let system = systems[system_index].lock().unwrap();
                let archetype_access = system.archetype_access();
                match system.thread_local_execution() {
                    ThreadLocalExecution::NextFlush => {
                        let resource_access = system.resource_access();
                        // if any system before this one conflicts, check all systems that came before for compatibility
                        if !current_archetype_access.is_compatible(archetype_access)
                            || !current_resource_access.is_compatible(resource_access)
                        {
                            #[allow(clippy::needless_range_loop)]
                            for earlier_system_index in
                                prepare_system_index_range.start..system_index
                            {
                                let earlier_system = systems[earlier_system_index].lock().unwrap();

                                // due to how prepare ranges work, previous systems should all be "NextFlush"
                                debug_assert_eq!(
                                    earlier_system.thread_local_execution(),
                                    ThreadLocalExecution::NextFlush
                                );

                                // if earlier system is incompatible, make the current system dependent
                                if !earlier_system
                                    .archetype_access()
                                    .is_compatible(archetype_access)
                                    || !earlier_system
                                        .resource_access()
                                        .is_compatible(resource_access)
                                {
                                    self.system_dependents[earlier_system_index].push(system_index);
                                    self.system_dependencies[system_index]
                                        .insert(earlier_system_index);
                                }
                            }
                        }

                        current_archetype_access.union(archetype_access);
                        current_resource_access.union(resource_access);

                        if let Some(last_thread_local_index) = last_thread_local_index {
                            self.system_dependents[last_thread_local_index].push(system_index);
                            self.system_dependencies[system_index].insert(last_thread_local_index);
                        }
                    }
                    ThreadLocalExecution::Immediate => {
                        for earlier_system_index in prepare_system_index_range.start..system_index {
                            // treat all earlier systems as "incompatible" to ensure we run this thread local system exclusively
                            self.system_dependents[earlier_system_index].push(system_index);
                            self.system_dependencies[system_index].insert(earlier_system_index);
                        }
                    }
                }
            }
        }

        self.next_thread_local_index += 1;
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
            RunReadyType::Range(range) => {
                all = range;
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
        schedule_changed: bool,
    ) {
        // if the schedule has changed, clear executor state / fill it with new defaults
        if schedule_changed {
            self.system_dependencies.clear();
            self.system_dependencies
                .resize_with(systems.len(), || FixedBitSet::with_capacity(systems.len()));
            self.thread_local_system_indices = Vec::new();

            self.system_dependents.clear();
            self.system_dependents.resize(systems.len(), Vec::new());

            self.finished_systems.grow(systems.len());
            self.running_systems.grow(systems.len());

            for (system_index, system) in systems.iter().enumerate() {
                let system = system.lock().unwrap();
                if system.thread_local_execution() == ThreadLocalExecution::Immediate {
                    self.thread_local_system_indices.push(system_index);
                }
            }
        }

        self.next_thread_local_index = 0;
        self.prepare_to_next_thread_local(world, systems, schedule_changed);

        self.finished_systems.clear();
        self.running_systems.clear();

        let mut run_ready_result = RunReadyResult::Ok;
        let run_ready_system_index_range =
            if let Some(index) = self.thread_local_system_indices.get(0) {
                // if there is an upcoming thread local system, run up to (and including) it
                0..(*index + 1)
            } else {
                // if there are no upcoming thread local systems, run everything right now
                0..systems.len()
            };
        rayon::scope_fifo(|scope| {
            run_ready_result = self.run_ready_systems(
                systems,
                RunReadyType::Range(run_ready_system_index_range),
                scope,
                world,
                resources,
            );
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

                self.prepare_to_next_thread_local(world, systems, schedule_changed);

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

        self.last_archetypes_generation = world.archetypes_generation();
    }
}

#[cfg(test)]
mod tests {
    use super::ParallelExecutor;
    use crate::{
        resource::{Res, ResMut, Resources},
        schedule::Schedule,
        system::{IntoQuerySystem, IntoThreadLocalSystem, Query},
        Commands,
    };
    use bevy_hecs::{Entity, World};
    use fixedbitset::FixedBitSet;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Counter {
        count: Arc<Mutex<usize>>,
    }

    #[test]
    fn cross_stage_archetype_change_prepare() {
        let mut world = World::new();
        let mut resources = Resources::default();
        let mut schedule = Schedule::default();
        schedule.add_stage("PreArchetypeChange");
        schedule.add_stage("PostArchetypeChange");

        fn insert(mut commands: Commands) {
            commands.spawn((1u32,));
        }

        fn read(query: Query<&u32>, mut entities: Query<Entity>) {
            for entity in &mut entities.iter() {
                // query.get() does a "system permission check" that will fail if the entity is from a
                // new archetype which hasnt been "prepared yet"
                query.get::<u32>(entity).unwrap();
            }

            assert_eq!(1, entities.iter().iter().count());
        }

        schedule.add_system_to_stage("PreArchetypeChange", insert.system());
        schedule.add_system_to_stage("PostArchetypeChange", read.system());

        let mut executor = ParallelExecutor::default();
        executor.run(&mut schedule, &mut world, &mut resources);
    }

    #[test]
    fn intra_stage_archetype_change_prepare() {
        let mut world = World::new();
        let mut resources = Resources::default();
        let mut schedule = Schedule::default();
        schedule.add_stage("update");

        fn insert(world: &mut World, _resources: &mut Resources) {
            world.spawn((1u32,));
        }

        fn read(query: Query<&u32>, mut entities: Query<Entity>) {
            for entity in &mut entities.iter() {
                // query.get() does a "system permission check" that will fail if the entity is from a
                // new archetype which hasnt been "prepared yet"
                query.get::<u32>(entity).unwrap();
            }

            assert_eq!(1, entities.iter().iter().count());
        }

        schedule.add_system_to_stage("update", insert.thread_local_system());
        schedule.add_system_to_stage("update", read.system());

        let mut executor = ParallelExecutor::default();
        executor.run(&mut schedule, &mut world, &mut resources);
    }

    #[test]
    fn schedule() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Counter::default());
        resources.insert(1.0f64);
        resources.insert(2isize);

        world.spawn((1.0f32,));
        world.spawn((1u32, 1u64));
        world.spawn((2u32,));

        let mut schedule = Schedule::default();
        schedule.add_stage("A"); // component queries
        schedule.add_stage("B"); // thread local
        schedule.add_stage("C"); // resources

        // A systems

        fn read_u32(counter: Res<Counter>, _query: Query<&u32>) {
            let mut count = counter.count.lock().unwrap();
            assert!(*count < 2, "should be one of the first two systems to run");
            *count += 1;
        }

        fn write_float(counter: Res<Counter>, _query: Query<&f32>) {
            let mut count = counter.count.lock().unwrap();
            assert!(*count < 2, "should be one of the first two systems to run");
            *count += 1;
        }

        fn read_u32_write_u64(counter: Res<Counter>, _query: Query<(&u32, &mut u64)>) {
            let mut count = counter.count.lock().unwrap();
            assert_eq!(*count, 2, "should always be the 3rd system to run");
            *count += 1;
        }

        fn read_u64(counter: Res<Counter>, _query: Query<&u64>) {
            let mut count = counter.count.lock().unwrap();
            assert_eq!(*count, 3, "should always be the 4th system to run");
            *count += 1;
        }

        schedule.add_system_to_stage("A", read_u32.system());
        schedule.add_system_to_stage("A", write_float.system());
        schedule.add_system_to_stage("A", read_u32_write_u64.system());
        schedule.add_system_to_stage("A", read_u64.system());

        // B systems

        fn write_u64(counter: Res<Counter>, _query: Query<&mut u64>) {
            let mut count = counter.count.lock().unwrap();
            assert_eq!(*count, 4, "should always be the 5th system to run");
            *count += 1;
        }

        fn thread_local_system(_world: &mut World, resources: &mut Resources) {
            let counter = resources.get::<Counter>().unwrap();
            let mut count = counter.count.lock().unwrap();
            assert_eq!(*count, 5, "should always be the 6th system to run");
            *count += 1;
        }

        fn write_f32(counter: Res<Counter>, _query: Query<&mut f32>) {
            let mut count = counter.count.lock().unwrap();
            assert_eq!(*count, 6, "should always be the 7th system to run");
            *count += 1;
        }

        schedule.add_system_to_stage("B", write_u64.system());
        schedule.add_system_to_stage("B", thread_local_system.thread_local_system());
        schedule.add_system_to_stage("B", write_f32.system());

        // C systems

        fn read_f64_res(counter: Res<Counter>, _f64_res: Res<f64>) {
            let mut count = counter.count.lock().unwrap();
            assert!(
                7 == *count || *count == 8,
                "should always be the 8th or 9th system to run"
            );
            *count += 1;
        }

        fn read_isize_res(counter: Res<Counter>, _isize_res: Res<isize>) {
            let mut count = counter.count.lock().unwrap();
            assert!(
                7 == *count || *count == 8,
                "should always be the 8th or 9th system to run"
            );
            *count += 1;
        }

        fn read_isize_write_f64_res(
            counter: Res<Counter>,
            _isize_res: Res<isize>,
            _f64_res: ResMut<f64>,
        ) {
            let mut count = counter.count.lock().unwrap();
            assert_eq!(*count, 9, "should always be the 10th system to run");
            *count += 1;
        }

        fn write_f64_res(counter: Res<Counter>, _f64_res: ResMut<f64>) {
            let mut count = counter.count.lock().unwrap();
            assert_eq!(*count, 10, "should always be the 11th system to run");
            *count += 1;
        }

        schedule.add_system_to_stage("C", read_f64_res.system());
        schedule.add_system_to_stage("C", read_isize_res.system());
        schedule.add_system_to_stage("C", read_isize_write_f64_res.system());
        schedule.add_system_to_stage("C", write_f64_res.system());

        fn run_executor_and_validate(
            executor: &mut ParallelExecutor,
            schedule: &mut Schedule,
            world: &mut World,
            resources: &mut Resources,
        ) {
            executor.run(schedule, world, resources);

            assert_eq!(
                executor.stages[0].system_dependents,
                vec![vec![2], vec![], vec![3], vec![]]
            );
            assert_eq!(
                executor.stages[1].system_dependents,
                vec![vec![1], vec![2], vec![]]
            );
            assert_eq!(
                executor.stages[2].system_dependents,
                vec![vec![2, 3], vec![], vec![3], vec![]]
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

            let stage_2_len = executor.stages[2].system_dependencies.len();
            let mut read_isize_write_f64_res_deps = FixedBitSet::with_capacity(stage_2_len);
            read_isize_write_f64_res_deps.insert(0);
            let mut write_f64_res_deps = FixedBitSet::with_capacity(stage_2_len);
            write_f64_res_deps.insert(0);
            write_f64_res_deps.insert(2);
            assert_eq!(
                executor.stages[2].system_dependencies,
                vec![
                    FixedBitSet::with_capacity(stage_2_len),
                    FixedBitSet::with_capacity(stage_2_len),
                    read_isize_write_f64_res_deps,
                    write_f64_res_deps
                ]
            );

            let counter = resources.get::<Counter>().unwrap();
            assert_eq!(
                *counter.count.lock().unwrap(),
                11,
                "counter should have been incremented once for each system"
            );
        }

        let mut executor = ParallelExecutor::default();
        run_executor_and_validate(&mut executor, &mut schedule, &mut world, &mut resources);
        // run again (with counter reset) to ensure executor works correctly across runs
        *resources.get::<Counter>().unwrap().count.lock().unwrap() = 0;
        run_executor_and_validate(&mut executor, &mut schedule, &mut world, &mut resources);
    }
}
