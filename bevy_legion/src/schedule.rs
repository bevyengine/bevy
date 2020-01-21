use crate::system::SystemId;
use crate::{
    borrow::RefMut, command::CommandBuffer, resource::ResourceTypeId, storage::ComponentTypeId,
    world::World,
};
use bit_set::BitSet;

#[cfg(feature = "par-schedule")]
use tracing::{span, trace, Level};

#[cfg(feature = "par-schedule")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "par-schedule")]
use fxhash::{FxHashMap, FxHashSet};

#[cfg(feature = "par-schedule")]
use rayon::prelude::*;

#[cfg(feature = "par-schedule")]
use itertools::izip;

#[cfg(feature = "par-schedule")]
use std::iter::repeat;

/// Empty trait which defines a `System` as schedulable by the dispatcher - this requires that the
/// type is both `Send` and `Sync`.
///
/// This is automatically implemented for all types that implement `Runnable` which meet the requirements.
pub trait Schedulable: Runnable + Send + Sync {}
impl<T> Schedulable for T where T: Runnable + Send + Sync {}

/// Describes which archetypes a system declares access to.
pub enum ArchetypeAccess {
    /// All archetypes.
    All,
    /// Some archetypes.
    Some(BitSet),
}

impl ArchetypeAccess {
    pub fn is_disjoint(&self, other: &ArchetypeAccess) -> bool {
        match self {
            Self::All => false,
            Self::Some(mine) => match other {
                Self::All => false,
                Self::Some(theirs) => mine.is_disjoint(theirs),
            },
        }
    }
}

/// Trait describing a schedulable type. This is implemented by `System`
pub trait Runnable {
    fn name(&self) -> &SystemId;
    fn reads(&self) -> (&[ResourceTypeId], &[ComponentTypeId]);
    fn writes(&self) -> (&[ResourceTypeId], &[ComponentTypeId]);
    fn prepare(&mut self, world: &World);
    fn accesses_archetypes(&self) -> &ArchetypeAccess;
    fn run(&self, world: &World);
    fn command_buffer_mut(&self) -> RefMut<CommandBuffer>;
}

/// Executes a sequence of systems, potentially in parallel, and then commits their command buffers.
///
/// Systems are provided in execution order. When the `par-schedule` feature is enabled, the `Executor`
/// may run some systems in parallel. The order in which side-effects (e.g. writes to resources
/// or entities) are observed is maintained.
pub struct Executor {
    systems: Vec<Box<dyn Schedulable>>,
    #[cfg(feature = "par-schedule")]
    static_dependants: Vec<Vec<usize>>,
    #[cfg(feature = "par-schedule")]
    dynamic_dependants: Vec<Vec<usize>>,
    #[cfg(feature = "par-schedule")]
    static_dependency_counts: Vec<AtomicUsize>,
    #[cfg(feature = "par-schedule")]
    awaiting: Vec<AtomicUsize>,
}

impl Executor {
    /// Constructs a new executor for all systems to be run in a single stage.
    ///
    /// Systems are provided in the order in which side-effects (e.g. writes to resources or entities)
    /// are to be observed.
    #[cfg(not(feature = "par-schedule"))]
    pub fn new(systems: Vec<Box<dyn Schedulable>>) -> Self { Self { systems } }

    /// Constructs a new executor for all systems to be run in a single stage.
    ///
    /// Systems are provided in the order in which side-effects (e.g. writes to resources or entities)
    /// are to be observed.
    #[cfg(feature = "par-schedule")]
    #[allow(clippy::cognitive_complexity)]
    // TODO: we should break this up
    pub fn new(systems: Vec<Box<dyn Schedulable>>) -> Self {
        if systems.len() > 1 {
            let mut static_dependency_counts = Vec::with_capacity(systems.len());

            let mut static_dependants: Vec<Vec<_>> =
                repeat(Vec::with_capacity(64)).take(systems.len()).collect();
            let mut dynamic_dependants: Vec<Vec<_>> =
                repeat(Vec::with_capacity(64)).take(systems.len()).collect();

            let mut resource_last_mutated =
                FxHashMap::<ResourceTypeId, usize>::with_capacity_and_hasher(
                    64,
                    Default::default(),
                );
            let mut resource_last_read =
                FxHashMap::<ResourceTypeId, usize>::with_capacity_and_hasher(
                    64,
                    Default::default(),
                );
            let mut component_mutated =
                FxHashMap::<ComponentTypeId, Vec<usize>>::with_capacity_and_hasher(
                    64,
                    Default::default(),
                );

            for (i, system) in systems.iter().enumerate() {
                let span = span!(
                    Level::TRACE,
                    "Building system dependencies",
                    system = %system.name(),
                    index = i,
                );
                let _guard = span.enter();

                let (read_res, read_comp) = system.reads();
                let (write_res, write_comp) = system.writes();

                // find resource access dependencies
                let mut dependencies = FxHashSet::with_capacity_and_hasher(64, Default::default());
                for res in read_res {
                    trace!(resource = ?res, "Read resource");
                    if let Some(n) = resource_last_mutated.get(res) {
                        trace!(system_index = n, "Added write dependency");
                        dependencies.insert(*n);
                    }
                    resource_last_read.insert(*res, i);
                }
                for res in write_res {
                    trace!(resource = ?res, "Write resource");
                    // Writes have to be exclusive, so we are dependent on reads too
                    if let Some(n) = resource_last_read.get(res) {
                        trace!(system_index = n, "Added read dependency");
                        dependencies.insert(*n);
                    }

                    if let Some(n) = resource_last_mutated.get(res) {
                        trace!(system_index = n, "Added write dependency");
                        dependencies.insert(*n);
                    }

                    resource_last_mutated.insert(*res, i);
                }

                static_dependency_counts.push(AtomicUsize::from(dependencies.len()));
                trace!(dependants = ?dependencies, "Computed static dependants");
                for dep in dependencies {
                    static_dependants[dep].push(i);
                }

                // find component access dependencies
                let mut comp_dependencies = FxHashSet::default();
                for comp in read_comp {
                    if let Some(ns) = component_mutated.get(comp) {
                        for n in ns {
                            comp_dependencies.insert(*n);
                        }
                    }
                }
                for comp in write_comp {
                    if let Some(ns) = component_mutated.get(comp) {
                        for n in ns {
                            comp_dependencies.insert(*n);
                        }
                    }
                    component_mutated
                        .entry(*comp)
                        .or_insert_with(Vec::new)
                        .push(i);
                }

                trace!(depentants = ?comp_dependencies, "Computed dynamic dependants");
                for dep in comp_dependencies {
                    if dep != i {
                        // dont be dependent on ourselves
                        dynamic_dependants[dep].push(i);
                    }
                }
            }

            trace!(
                ?static_dependants,
                ?dynamic_dependants,
                "Computed system dependencies"
            );

            let mut awaiting = Vec::with_capacity(systems.len());
            systems
                .iter()
                .for_each(|_| awaiting.push(AtomicUsize::new(0)));

            Executor {
                awaiting,
                static_dependants,
                dynamic_dependants,
                static_dependency_counts,
                systems,
            }
        } else {
            Executor {
                awaiting: Vec::with_capacity(0),
                static_dependants: Vec::with_capacity(0),
                dynamic_dependants: Vec::with_capacity(0),
                static_dependency_counts: Vec::with_capacity(0),
                systems,
            }
        }
    }

    /// Converts this executor into a vector of its component systems.
    pub fn into_vec(self) -> Vec<Box<dyn Schedulable>> { self.systems }

    /// Executes all systems and then flushes their command buffers.
    pub fn execute(&mut self, world: &mut World) {
        self.run_systems(world);
        self.flush_command_buffers(world);
    }

    /// Executes all systems sequentially.
    ///
    /// Only enabled with par-schedule is disabled
    #[cfg(not(feature = "par-schedule"))]
    pub fn run_systems(&mut self, world: &mut World) {
        // preflush command buffers
        // This also handles the first case of allocating them.
        self.systems
            .iter()
            .for_each(|system| system.command_buffer_mut().write(world));

        self.systems.iter_mut().for_each(|system| {
            system.run(world);
        });
    }

    /// Executes all systems, potentially in parallel.
    ///
    /// Ordering is retained in so far as the order of observed resource and component
    /// accesses is maintained.
    ///
    /// Call from within `rayon::ThreadPool::install()` to execute within a specific thread pool.
    #[cfg(feature = "par-schedule")]
    pub fn run_systems(&mut self, world: &mut World) {
        // preflush command buffers
        // This also handles the first case of allocating them.
        self.systems
            .iter()
            .for_each(|system| system.command_buffer_mut().write(world));

        rayon::join(
            || {},
            || {
                match self.systems.len() {
                    1 => {
                        self.systems[0].run(world);
                    }
                    _ => {
                        let systems = &mut self.systems;
                        let static_dependency_counts = &self.static_dependency_counts;
                        let awaiting = &mut self.awaiting;

                        // prepare all systems - archetype filters are pre-executed here
                        systems.par_iter_mut().for_each(|sys| sys.prepare(world));

                        // determine dynamic dependencies
                        izip!(
                            systems.iter(),
                            self.static_dependants.iter_mut(),
                            self.dynamic_dependants.iter_mut()
                        )
                        .par_bridge()
                        .for_each(|(sys, static_dep, dyn_dep)| {
                            let archetypes = sys.accesses_archetypes();
                            for i in (0..dyn_dep.len()).rev() {
                                let dep = dyn_dep[i];
                                let other = &systems[dep];

                                // if the archetype sets intersect,
                                // then we can move the dynamic dependant into the static dependants set
                                if !other.accesses_archetypes().is_disjoint(archetypes) {
                                    static_dep.push(dep);
                                    dyn_dep.swap_remove(i);
                                    static_dependency_counts[dep].fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        });

                        // initialize dependency tracking
                        for (i, count) in static_dependency_counts.iter().enumerate() {
                            awaiting[i].store(count.load(Ordering::Relaxed), Ordering::Relaxed);
                        }

                        let awaiting = &self.awaiting;

                        // execute all systems with no outstanding dependencies
                        (0..systems.len())
                            .filter(|i| awaiting[*i].load(Ordering::SeqCst) == 0)
                            .for_each(|i| {
                                self.run_recursive(i, world);
                            });
                    }
                }
            },
        );
    }

    /// Flushes the recorded command buffers for all systems.
    pub fn flush_command_buffers(&mut self, world: &mut World) {
        self.systems.iter().for_each(|system| {
            system.command_buffer_mut().write(world);
        });
    }

    /// Recursively execute through the generated depedency cascade and exhaust it.
    #[cfg(feature = "par-schedule")]
    fn run_recursive(&self, i: usize, world: &World) {
        self.systems[i].run(world);

        self.static_dependants[i].par_iter().for_each(|dep| {
            match self.awaiting[*dep].compare_exchange(
                1,
                std::usize::MAX,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.run_recursive(*dep, world);
                }
                Err(_) => {
                    self.awaiting[*dep].fetch_sub(1, Ordering::Relaxed);
                }
            }
        });
    }
}

/// A factory for `Schedule`.
pub struct Builder {
    steps: Vec<Step>,
    accumulator: Vec<Box<dyn Schedulable>>,
}

impl Builder {
    /// Adds a system to the schedule.
    pub fn add_system<T: Into<Box<dyn Schedulable>>>(mut self, system: T) -> Self {
        self.accumulator.push(system.into());
        self
    }

    /// Waits for executing systems to complete, and the flushes all outstanding system
    /// command buffers.
    pub fn flush(mut self) -> Self {
        self.finalize_executor();
        self.steps.push(Step::FlushCmdBuffers);
        self
    }

    fn finalize_executor(&mut self) {
        if !self.accumulator.is_empty() {
            let mut systems = Vec::new();
            std::mem::swap(&mut self.accumulator, &mut systems);
            let executor = Executor::new(systems);
            self.steps.push(Step::Systems(executor));
        }
    }

    /// Adds a thread local function to the schedule. This function will be executed on the main thread.
    pub fn add_thread_local_fn<F: FnMut(&mut World) + 'static>(mut self, f: F) -> Self {
        self.finalize_executor();
        self.steps.push(Step::ThreadLocalFn(
            Box::new(f) as Box<dyn FnMut(&mut World)>
        ));
        self
    }

    /// Adds a thread local system to the schedule. This system will be executed on the main thread.
    pub fn add_thread_local<S: Into<Box<dyn Runnable>>>(self, system: S) -> Self {
        let system = system.into();
        self.add_thread_local_fn(move |world| system.run(world))
    }

    /// Finalizes the builder into a `Schedule`.
    pub fn build(self) -> Schedule { self.into() }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            accumulator: Vec::new(),
        }
    }
}

/// A step in a schedule.
pub enum Step {
    /// A batch of systems.
    Systems(Executor),
    /// Flush system command buffers.
    FlushCmdBuffers,
    /// A thread local function.
    ThreadLocalFn(Box<dyn FnMut(&mut World)>),
}

/// A schedule of systems for execution.
///
/// # Examples
///
/// ```rust
/// # use legion::prelude::*;
/// # let find_collisions = SystemBuilder::new("find_collisions").build(|_,_,_,_| {});
/// # let calculate_acceleration = SystemBuilder::new("calculate_acceleration").build(|_,_,_,_| {});
/// # let update_positions = SystemBuilder::new("update_positions").build(|_,_,_,_| {});
/// # let mut world = World::new();
/// let mut schedule = Schedule::builder()
///     .add_system(find_collisions)
///     .flush()
///     .add_system(calculate_acceleration)
///     .add_system(update_positions)
///     .build();
///
/// schedule.execute(&mut world);
/// ```
pub struct Schedule {
    steps: Vec<Step>,
}

impl Schedule {
    /// Creates a new schedule builder.
    pub fn builder() -> Builder { Builder::default() }

    /// Executes all of the steps in the schedule.
    pub fn execute(&mut self, world: &mut World) {
        let mut waiting_flush: Vec<&mut Executor> = Vec::new();
        for step in &mut self.steps {
            match step {
                Step::Systems(executor) => {
                    executor.run_systems(world);
                    waiting_flush.push(executor);
                }
                Step::FlushCmdBuffers => waiting_flush
                    .drain(..)
                    .for_each(|e| e.flush_command_buffers(world)),
                Step::ThreadLocalFn(function) => function(world),
            }
        }
    }

    /// Converts the schedule into a vector of steps.
    pub fn into_vec(self) -> Vec<Step> { self.steps }
}

impl From<Builder> for Schedule {
    fn from(builder: Builder) -> Self {
        Self {
            steps: builder.flush().steps,
        }
    }
}

impl From<Vec<Step>> for Schedule {
    fn from(steps: Vec<Step>) -> Self { Self { steps } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use itertools::sorted;
    use std::sync::{Arc, Mutex};

    #[test]
    fn execute_in_order() {
        let universe = Universe::new();
        let mut world = universe.create_world();

        #[derive(Default)]
        struct Resource;

        world.resources.insert(Resource);

        let order = Arc::new(Mutex::new(Vec::new()));

        let order_clone = order.clone();
        let system_one = SystemBuilder::new("one")
            .write_resource::<Resource>()
            .build(move |_, _, _, _| order_clone.lock().unwrap().push(1usize));
        let order_clone = order.clone();
        let system_two = SystemBuilder::new("two")
            .write_resource::<Resource>()
            .build(move |_, _, _, _| order_clone.lock().unwrap().push(2usize));
        let order_clone = order.clone();
        let system_three = SystemBuilder::new("three")
            .write_resource::<Resource>()
            .build(move |_, _, _, _| order_clone.lock().unwrap().push(3usize));

        let mut schedule = Schedule::builder()
            .add_system(system_one)
            .add_system(system_two)
            .add_system(system_three)
            .build();

        schedule.execute(&mut world);

        let order = order.lock().unwrap();
        let sorted: Vec<usize> = sorted(order.clone()).collect();
        assert_eq!(*order, sorted);
    }

    #[test]
    fn flush() {
        let universe = Universe::new();
        let mut world = universe.create_world();

        #[derive(Clone, Copy, Debug, PartialEq)]
        struct TestComp(f32, f32, f32);

        let system_one = SystemBuilder::new("one").build(move |cmd, _, _, _| {
            cmd.insert((), vec![(TestComp(0., 0., 0.),)]).unwrap();
        });
        let system_two = SystemBuilder::new("two")
            .with_query(Write::<TestComp>::query())
            .build(move |_, world, _, query| assert_eq!(0, query.iter_mut(world).count()));
        let system_three = SystemBuilder::new("three")
            .with_query(Write::<TestComp>::query())
            .build(move |_, world, _, query| assert_eq!(1, query.iter_mut(world).count()));

        let mut schedule = Schedule::builder()
            .add_system(system_one)
            .add_system(system_two)
            .flush()
            .add_system(system_three)
            .build();

        schedule.execute(&mut world);
    }
}
