mod stage;
mod stage_executor;
mod state;

pub use stage::*;
pub use stage_executor::*;
pub use state::*;

use crate::{BoxedSystem, IntoSystem, Resources, System, World};
use bevy_utils::HashMap;

#[derive(Default)]
pub struct Schedule {
    stages: HashMap<String, Box<dyn Stage>>,
    stage_order: Vec<String>,
    run_criteria: Option<BoxedSystem<(), ShouldRun>>,
    run_criteria_initialized: bool,
}

impl Schedule {
    pub fn with_stage<S: Stage>(mut self, name: &str, stage: S) -> Self {
        self.add_stage(name, stage);
        self
    }

    pub fn with_stage_after<S: Stage>(mut self, target: &str, name: &str, stage: S) -> Self {
        self.add_stage_after(target, name, stage);
        self
    }

    pub fn with_stage_before<S: Stage>(mut self, target: &str, name: &str, stage: S) -> Self {
        self.add_stage_before(target, name, stage);
        self
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.set_run_criteria(system);
        self
    }

    pub fn with_system_in_stage<S: System<In = (), Out = ()>>(
        mut self,
        stage_name: &'static str,
        system: S,
    ) -> Self {
        self.add_system_to_stage(stage_name, system);
        self
    }

    pub fn set_run_criteria<S: System<In = (), Out = ShouldRun>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.run_criteria = Some(Box::new(system.system()));
        self.run_criteria_initialized = false;
        self
    }

    pub fn add_stage<S: Stage>(&mut self, name: &str, stage: S) -> &mut Self {
        self.stage_order.push(name.to_string());
        self.stages.insert(name.to_string(), Box::new(stage));
        self
    }

    pub fn add_stage_after<S: Stage>(&mut self, target: &str, name: &str, stage: S) -> &mut Self {
        if self.stages.get(name).is_some() {
            panic!("Stage already exists: {}.", name);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_name)| *stage_name == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}.", target));

        self.stages.insert(name.to_string(), Box::new(stage));
        self.stage_order.insert(target_index + 1, name.to_string());
        self
    }

    pub fn add_stage_before<S: Stage>(&mut self, target: &str, name: &str, stage: S) -> &mut Self {
        if self.stages.get(name).is_some() {
            panic!("Stage already exists: {}.", name);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_name)| *stage_name == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}.", target));

        self.stages.insert(name.to_string(), Box::new(stage));
        self.stage_order.insert(target_index, name.to_string());
        self
    }

    pub fn add_system_to_stage<S: System<In = (), Out = ()>>(
        &mut self,
        stage_name: &'static str,
        system: S,
    ) -> &mut Self {
        let stage = self
            .get_stage_mut::<SystemStage>(stage_name)
            .unwrap_or_else(|| {
                panic!(
                    "Stage '{}' does not exist or is not a SystemStage",
                    stage_name
                )
            });
        stage.add_system(system.system());
        self
    }

    pub fn stage<T: Stage, F: FnOnce(&mut T) -> &mut T>(
        &mut self,
        name: &str,
        func: F,
    ) -> &mut Self {
        let stage = self
            .get_stage_mut::<T>(name)
            .unwrap_or_else(|| panic!("stage '{}' does not exist or is the wrong type", name));
        func(stage);
        self
    }

    pub fn get_stage<T: Stage>(&self, name: &str) -> Option<&T> {
        self.stages
            .get(name)
            .and_then(|stage| stage.downcast_ref::<T>())
    }

    pub fn get_stage_mut<T: Stage>(&mut self, name: &str) -> Option<&mut T> {
        self.stages
            .get_mut(name)
            .and_then(|stage| stage.downcast_mut::<T>())
    }

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        for name in self.stage_order.iter() {
            #[cfg(feature = "trace")]
            let stage_span = bevy_utils::tracing::info_span!("stage", name = name.as_str());
            #[cfg(feature = "trace")]
            let _stage_guard = stage_span.enter();
            let stage = self.stages.get_mut(name).unwrap();
            stage.run(world, resources);
        }
    }

    /// Shorthand for [Schedule::initialize] and [Schedule::run]
    pub fn initialize_and_run(&mut self, world: &mut World, resources: &mut Resources) {
        self.initialize(world, resources);
        self.run(world, resources);
    }
}

impl Stage for Schedule {
    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        if let Some(ref mut run_criteria) = self.run_criteria {
            if !self.run_criteria_initialized {
                run_criteria.initialize(world, resources);
                self.run_criteria_initialized = true;
            }
        }

        for name in self.stage_order.iter() {
            let stage = self.stages.get_mut(name).unwrap();
            stage.initialize(world, resources);
        }
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        loop {
            let should_run = if let Some(ref mut run_criteria) = self.run_criteria {
                let should_run = run_criteria.run((), world, resources);
                run_criteria.run_thread_local(world, resources);
                // don't run when no result is returned or false is returned
                should_run.unwrap_or(ShouldRun::No)
            } else {
                ShouldRun::Yes
            };

            match should_run {
                ShouldRun::No => return,
                ShouldRun::Yes => {
                    self.run_once(world, resources);
                    return;
                }
                ShouldRun::YesAndLoop => {
                    self.run_once(world, resources);
                }
            }
        }
    }
}

pub fn clear_trackers_system(world: &mut World, resources: &mut Resources) {
    world.clear_trackers();
    resources.clear_trackers();
}

#[cfg(test)]
mod tests {
    use crate::{
        resource::{Res, ResMut, Resources},
        schedule::{ParallelSystemStageExecutor, Schedule, SystemStage},
        system::Query,
        Commands, Entity, IntoSystem, World,
    };
    use bevy_tasks::{ComputeTaskPool, TaskPool};
    use fixedbitset::FixedBitSet;
    use parking_lot::Mutex;
    use std::{collections::HashSet, sync::Arc};

    #[derive(Default)]
    struct CompletedSystems {
        completed_systems: Arc<Mutex<HashSet<&'static str>>>,
    }

    #[test]
    fn cross_stage_archetype_change_prepare() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(ComputeTaskPool(TaskPool::default()));

        fn insert(commands: &mut Commands) {
            commands.spawn((1u32,));
        }

        fn read(query: Query<&u32>, entities: Query<Entity>) {
            for entity in &mut entities.iter() {
                // query.get() does a "system permission check" that will fail if the entity is from a
                // new archetype which hasnt been "prepared yet"
                query.get_component::<u32>(entity).unwrap();
            }

            assert_eq!(1, entities.iter().count());
        }

        let mut schedule = Schedule::default();
        let mut pre_archetype_change = SystemStage::parallel();
        pre_archetype_change.add_system(insert.system());
        schedule.add_stage("PreArchetypeChange", pre_archetype_change);
        let mut post_archetype_change = SystemStage::parallel();
        post_archetype_change.add_system(read.system());
        schedule.add_stage("PostArchetypeChange", post_archetype_change);

        schedule.initialize_and_run(&mut world, &mut resources);
    }

    #[test]
    fn intra_stage_archetype_change_prepare() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(ComputeTaskPool(TaskPool::default()));

        fn insert(world: &mut World, _resources: &mut Resources) {
            world.spawn((1u32,));
        }

        fn read(query: Query<&u32>, entities: Query<Entity>) {
            for entity in &mut entities.iter() {
                // query.get() does a "system permission check" that will fail if the entity is from a
                // new archetype which hasnt been "prepared yet"
                query.get_component::<u32>(entity).unwrap();
            }

            assert_eq!(1, entities.iter().count());
        }

        let mut update = SystemStage::parallel();
        update.add_system(insert.system());
        update.add_system(read.system());

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update);

        schedule.initialize_and_run(&mut world, &mut resources);
    }

    #[test]
    fn schedule() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(ComputeTaskPool(TaskPool::default()));
        resources.insert(CompletedSystems::default());
        resources.insert(1.0f64);
        resources.insert(2isize);

        world.spawn((1.0f32,));
        world.spawn((1u32, 1u64));
        world.spawn((2u32,));

        let mut stage_a = SystemStage::parallel(); // component queries
        let mut stage_b = SystemStage::parallel(); // thread local
        let mut stage_c = SystemStage::parallel(); // resources

        // A system names
        const READ_U32_SYSTEM_NAME: &str = "read_u32";
        const WRITE_FLOAT_SYSTEM_NAME: &str = "write_float";
        const READ_U32_WRITE_U64_SYSTEM_NAME: &str = "read_u32_write_u64";
        const READ_U64_SYSTEM_NAME: &str = "read_u64";

        // B system names
        const WRITE_U64_SYSTEM_NAME: &str = "write_u64";
        const THREAD_LOCAL_SYSTEM_SYSTEM_NAME: &str = "thread_local_system";
        const WRITE_F32_SYSTEM_NAME: &str = "write_f32";

        // C system names
        const READ_F64_RES_SYSTEM_NAME: &str = "read_f64_res";
        const READ_ISIZE_RES_SYSTEM_NAME: &str = "read_isize_res";
        const READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME: &str = "read_isize_write_f64_res";
        const WRITE_F64_RES_SYSTEM_NAME: &str = "write_f64_res";

        // A systems

        fn read_u32(completed_systems: Res<CompletedSystems>, _query: Query<&u32>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            completed_systems.insert(READ_U32_SYSTEM_NAME);
        }

        fn write_float(completed_systems: Res<CompletedSystems>, _query: Query<&f32>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            completed_systems.insert(WRITE_FLOAT_SYSTEM_NAME);
        }

        fn read_u32_write_u64(
            completed_systems: Res<CompletedSystems>,
            _query: Query<(&u32, &mut u64)>,
        ) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(!completed_systems.contains(READ_U64_SYSTEM_NAME));
            completed_systems.insert(READ_U32_WRITE_U64_SYSTEM_NAME);
        }

        fn read_u64(completed_systems: Res<CompletedSystems>, _query: Query<&u64>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(completed_systems.contains(READ_U32_WRITE_U64_SYSTEM_NAME));
            assert!(!completed_systems.contains(WRITE_U64_SYSTEM_NAME));
            completed_systems.insert(READ_U64_SYSTEM_NAME);
        }

        stage_a.add_system(read_u32.system());
        stage_a.add_system(write_float.system());
        stage_a.add_system(read_u32_write_u64.system());
        stage_a.add_system(read_u64.system());

        // B systems

        fn write_u64(completed_systems: Res<CompletedSystems>, _query: Query<&mut u64>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(completed_systems.contains(READ_U64_SYSTEM_NAME));
            assert!(!completed_systems.contains(THREAD_LOCAL_SYSTEM_SYSTEM_NAME));
            assert!(!completed_systems.contains(WRITE_F32_SYSTEM_NAME));
            completed_systems.insert(WRITE_U64_SYSTEM_NAME);
        }

        fn thread_local_system(_world: &mut World, resources: &mut Resources) {
            let completed_systems = resources.get::<CompletedSystems>().unwrap();
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(completed_systems.contains(WRITE_U64_SYSTEM_NAME));
            assert!(!completed_systems.contains(WRITE_F32_SYSTEM_NAME));
            completed_systems.insert(THREAD_LOCAL_SYSTEM_SYSTEM_NAME);
        }

        fn write_f32(completed_systems: Res<CompletedSystems>, _query: Query<&mut f32>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(completed_systems.contains(WRITE_U64_SYSTEM_NAME));
            assert!(completed_systems.contains(THREAD_LOCAL_SYSTEM_SYSTEM_NAME));
            assert!(!completed_systems.contains(READ_F64_RES_SYSTEM_NAME));
            completed_systems.insert(WRITE_F32_SYSTEM_NAME);
        }

        stage_b.add_system(write_u64.system());
        stage_b.add_system(thread_local_system.system());
        stage_b.add_system(write_f32.system());

        // C systems

        fn read_f64_res(completed_systems: Res<CompletedSystems>, _f64_res: Res<f64>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(completed_systems.contains(WRITE_F32_SYSTEM_NAME));
            assert!(!completed_systems.contains(READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME));
            assert!(!completed_systems.contains(WRITE_F64_RES_SYSTEM_NAME));
            completed_systems.insert(READ_F64_RES_SYSTEM_NAME);
        }

        fn read_isize_res(completed_systems: Res<CompletedSystems>, _isize_res: Res<isize>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            completed_systems.insert(READ_ISIZE_RES_SYSTEM_NAME);
        }

        fn read_isize_write_f64_res(
            completed_systems: Res<CompletedSystems>,
            _isize_res: Res<isize>,
            _f64_res: ResMut<f64>,
        ) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(completed_systems.contains(READ_F64_RES_SYSTEM_NAME));
            assert!(!completed_systems.contains(WRITE_F64_RES_SYSTEM_NAME));
            completed_systems.insert(READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME);
        }

        fn write_f64_res(completed_systems: Res<CompletedSystems>, _f64_res: ResMut<f64>) {
            let mut completed_systems = completed_systems.completed_systems.lock();
            assert!(completed_systems.contains(READ_F64_RES_SYSTEM_NAME));
            assert!(completed_systems.contains(READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME));
            completed_systems.insert(WRITE_F64_RES_SYSTEM_NAME);
        }

        stage_c.add_system(read_f64_res.system());
        stage_c.add_system(read_isize_res.system());
        stage_c.add_system(read_isize_write_f64_res.system());
        stage_c.add_system(write_f64_res.system());

        fn run_and_validate(schedule: &mut Schedule, world: &mut World, resources: &mut Resources) {
            schedule.initialize_and_run(world, resources);

            let stage_a = schedule.get_stage::<SystemStage>("a").unwrap();
            let stage_b = schedule.get_stage::<SystemStage>("b").unwrap();
            let stage_c = schedule.get_stage::<SystemStage>("c").unwrap();

            let a_executor = stage_a
                .get_executor::<ParallelSystemStageExecutor>()
                .unwrap();
            let b_executor = stage_b
                .get_executor::<ParallelSystemStageExecutor>()
                .unwrap();
            let c_executor = stage_c
                .get_executor::<ParallelSystemStageExecutor>()
                .unwrap();

            assert_eq!(
                a_executor.system_dependents(),
                vec![vec![], vec![], vec![3], vec![]]
            );
            assert_eq!(
                b_executor.system_dependents(),
                vec![vec![1], vec![2], vec![]]
            );
            assert_eq!(
                c_executor.system_dependents(),
                vec![vec![2, 3], vec![], vec![3], vec![]]
            );

            let stage_a_len = a_executor.system_dependencies().len();
            let mut read_u64_deps = FixedBitSet::with_capacity(stage_a_len);
            read_u64_deps.insert(2);

            assert_eq!(
                a_executor.system_dependencies(),
                vec![
                    FixedBitSet::with_capacity(stage_a_len),
                    FixedBitSet::with_capacity(stage_a_len),
                    FixedBitSet::with_capacity(stage_a_len),
                    read_u64_deps,
                ]
            );

            let stage_b_len = b_executor.system_dependencies().len();
            let mut thread_local_deps = FixedBitSet::with_capacity(stage_b_len);
            thread_local_deps.insert(0);
            let mut write_f64_deps = FixedBitSet::with_capacity(stage_b_len);
            write_f64_deps.insert(1);
            assert_eq!(
                b_executor.system_dependencies(),
                vec![
                    FixedBitSet::with_capacity(stage_b_len),
                    thread_local_deps,
                    write_f64_deps
                ]
            );

            let stage_c_len = c_executor.system_dependencies().len();
            let mut read_isize_write_f64_res_deps = FixedBitSet::with_capacity(stage_c_len);
            read_isize_write_f64_res_deps.insert(0);
            let mut write_f64_res_deps = FixedBitSet::with_capacity(stage_c_len);
            write_f64_res_deps.insert(0);
            write_f64_res_deps.insert(2);
            assert_eq!(
                c_executor.system_dependencies(),
                vec![
                    FixedBitSet::with_capacity(stage_c_len),
                    FixedBitSet::with_capacity(stage_c_len),
                    read_isize_write_f64_res_deps,
                    write_f64_res_deps
                ]
            );

            let completed_systems = resources.get::<CompletedSystems>().unwrap();
            assert_eq!(
                completed_systems.completed_systems.lock().len(),
                11,
                "completed_systems should have been incremented once for each system"
            );
        }

        let mut schedule = Schedule::default();
        schedule.add_stage("a", stage_a);
        schedule.add_stage("b", stage_b);
        schedule.add_stage("c", stage_c);

        // Test the "clean start" case
        run_and_validate(&mut schedule, &mut world, &mut resources);

        // Stress test the "continue running" case
        for _ in 0..1000 {
            // run again (with completed_systems reset) to ensure executor works correctly across runs
            resources
                .get::<CompletedSystems>()
                .unwrap()
                .completed_systems
                .lock()
                .clear();
            run_and_validate(&mut schedule, &mut world, &mut resources);
        }
    }
}
