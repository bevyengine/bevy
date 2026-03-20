//! Explicit-access scheduling APIs intended for agent-driven workflows.

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use bevy_tasks::{block_on, BoxedFuture};
use core::future::Future;

use crate::{
    component::{Component, ComponentId},
    query::{ComponentAccessKind, FilteredAccessSet},
    resource::Resource,
    system::{IntoSystem, ScheduleSystem},
    world::{World, WorldId},
};

/// Controls whether declared access is trusted or verified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccessTrustMode {
    /// Trust the declared access and skip verification.
    Trusted,
    /// Verify that the declaration is a superset of a verifier system's access.
    #[default]
    Verify,
}

#[derive(Clone)]
enum AccessResolver {
    Component(Arc<dyn Fn(&mut World) -> ComponentId + Send + Sync>),
    Resource(Arc<dyn Fn(&mut World) -> ComponentId + Send + Sync>),
}

impl AccessResolver {
    fn resolve(&self, world: &mut World) -> ComponentId {
        match self {
            Self::Component(resolve) | Self::Resource(resolve) => resolve(world),
        }
    }
}

/// A declarative access footprint for an agent system.
#[derive(Clone, Default)]
pub struct AccessDeclaration {
    component_reads: Vec<AccessResolver>,
    component_writes: Vec<AccessResolver>,
    resource_reads: Vec<AccessResolver>,
    resource_writes: Vec<AccessResolver>,
    /// Whether this system requires exclusive world access when applying results.
    pub exclusive: bool,
    /// Whether this system must marshal its results on the main thread.
    pub main_thread: bool,
    /// Whether this system is allowed to execute asynchronously.
    pub async_allowed: bool,
    /// Priority used when compiling deterministic execution order.
    pub priority: i32,
    /// Whether the declaration is trusted or verified.
    pub trust_mode: AccessTrustMode,
}

impl AccessDeclaration {
    /// Creates an empty declaration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares shared component access.
    pub fn read_component<T: Component>(mut self) -> Self {
        self.component_reads
            .push(AccessResolver::Component(Arc::new(|world| {
                world.register_component::<T>()
            })));
        self
    }

    /// Declares exclusive component access.
    pub fn write_component<T: Component>(mut self) -> Self {
        self.component_writes
            .push(AccessResolver::Component(Arc::new(|world| {
                world.register_component::<T>()
            })));
        self
    }

    /// Declares shared resource access.
    pub fn read_resource<T: Resource>(mut self) -> Self {
        self.resource_reads
            .push(AccessResolver::Resource(Arc::new(|world| {
                world.register_resource::<T>()
            })));
        self
    }

    /// Declares exclusive resource access.
    pub fn write_resource<T: Resource>(mut self) -> Self {
        self.resource_writes
            .push(AccessResolver::Resource(Arc::new(|world| {
                world.register_resource::<T>()
            })));
        self
    }

    /// Sets the priority used at compile time.
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Marks the declaration as requiring exclusive application.
    pub fn exclusive(mut self) -> Self {
        self.exclusive = true;
        self
    }

    /// Marks the declaration as requiring main-thread application.
    pub fn main_thread(mut self) -> Self {
        self.main_thread = true;
        self
    }

    /// Allows async execution for this declaration.
    pub fn async_allowed(mut self) -> Self {
        self.async_allowed = true;
        self
    }

    /// Skips verification for this declaration.
    pub fn trusted(mut self) -> Self {
        self.trust_mode = AccessTrustMode::Trusted;
        self
    }

    /// Verifies this declaration against a system access footprint.
    pub fn verify(mut self) -> Self {
        self.trust_mode = AccessTrustMode::Verify;
        self
    }

    /// Resolves this declaration into a Bevy access set.
    pub fn resolve(&self, world: &mut World) -> FilteredAccessSet {
        let mut access = FilteredAccessSet::new();
        for resolver in &self.component_reads {
            access.add_unfiltered_component_read(resolver.resolve(world));
        }
        for resolver in &self.component_writes {
            access.add_unfiltered_component_write(resolver.resolve(world));
        }
        for resolver in &self.resource_reads {
            access.add_resource_read(resolver.resolve(world));
        }
        for resolver in &self.resource_writes {
            access.add_resource_write(resolver.resolve(world));
        }
        if self.exclusive {
            access.write_all();
        }
        access
    }

    fn verifies(&self, world: &mut World, inferred: &FilteredAccessSet) -> bool {
        let declared = self.resolve(world);
        let declared = declared.combined_access();
        let inferred = inferred.combined_access();

        if inferred.has_read_all() && !declared.has_read_all() {
            return false;
        }
        if inferred.has_write_all() && !declared.has_write_all() {
            return false;
        }

        let Ok(accesses) = inferred.try_iter_access() else {
            return false;
        };

        accesses.into_iter().all(|access| match access {
            ComponentAccessKind::Shared(component_id) => {
                declared.has_read(component_id) || declared.has_write(component_id)
            }
            ComponentAccessKind::Exclusive(component_id) => declared.has_write(component_id),
            ComponentAccessKind::Archetypal(component_id) => {
                declared.has_read(component_id) || declared.has_write(component_id)
            }
        })
    }
}

/// A read-only snapshot extracted from a world for agent execution.
pub struct WorldSnapshot<T> {
    world_id: WorldId,
    data: Arc<T>,
}

impl<T> Clone for WorldSnapshot<T> {
    fn clone(&self) -> Self {
        Self {
            world_id: self.world_id,
            data: self.data.clone(),
        }
    }
}

impl<T> WorldSnapshot<T> {
    /// Returns the world this snapshot was extracted from.
    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Returns the inner snapshot data.
    pub fn view(&self) -> SnapshotView<'_, T> {
        self.data.as_ref()
    }
}

/// A read-only view into a world snapshot.
pub type SnapshotView<'a, T> = &'a T;

/// Deferred world mutations returned by agent systems.
#[derive(Default)]
pub struct AgentCommands {
    commands: Vec<Box<dyn FnOnce(&mut World) + Send + 'static>>,
}

impl AgentCommands {
    /// Queues an arbitrary world mutation.
    pub fn push(&mut self, command: impl FnOnce(&mut World) + Send + 'static) -> &mut Self {
        self.commands.push(Box::new(command));
        self
    }

    /// Queues a resource insertion.
    pub fn insert_resource<T: Resource + Send + 'static>(&mut self, value: T) -> &mut Self {
        self.push(move |world| {
            world.insert_resource(value);
        })
    }

    /// Applies queued mutations to the world.
    pub fn apply(self, world: &mut World) {
        for command in self.commands {
            command(world);
        }
    }
}

enum AgentRunner<T> {
    Sync(Box<dyn Fn(&WorldSnapshot<T>, &mut AgentCommands) + Send + Sync + 'static>),
    Async(
        Box<
            dyn Fn(WorldSnapshot<T>) -> BoxedFuture<'static, AgentCommands> + Send + Sync + 'static,
        >,
    ),
}

struct AgentSystem<T> {
    declaration: AccessDeclaration,
    runner: AgentRunner<T>,
    insertion_order: usize,
}

/// Errors returned while compiling or running an agent schedule.
#[derive(Debug, thiserror::Error)]
pub enum AgentScheduleError {
    /// Async execution was requested without enabling it in the declaration.
    #[error("agent system `{0}` is async but its declaration does not allow async execution")]
    AsyncNotAllowed(String),
    /// Verification was requested without a verifier system.
    #[error("agent system `{0}` requested verification but no verifier system was provided")]
    MissingVerifier(String),
    /// The declared access was not a superset of the verifier system access.
    #[error("agent system `{0}` declared an access footprint narrower than its verifier system")]
    AccessVerificationFailed(String),
}

/// Builder for explicit-access agent systems.
pub struct AgentSystemBuilder<T> {
    name: String,
    declaration: AccessDeclaration,
    verifier: Option<ScheduleSystem>,
    runner: AgentRunner<T>,
}

impl<T: Send + Sync + 'static> AgentSystemBuilder<T> {
    /// Creates a synchronous agent system.
    pub fn sync(
        name: impl Into<String>,
        declaration: AccessDeclaration,
        run: impl Fn(&WorldSnapshot<T>, &mut AgentCommands) + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            declaration,
            verifier: None,
            runner: AgentRunner::Sync(Box::new(run)),
        }
    }

    /// Creates an asynchronous agent system.
    pub fn asynchronous<F, Fut>(
        name: impl Into<String>,
        declaration: AccessDeclaration,
        run: F,
    ) -> Self
    where
        F: Fn(WorldSnapshot<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = AgentCommands> + Send + 'static,
    {
        Self {
            name: name.into(),
            declaration,
            verifier: None,
            runner: AgentRunner::Async(Box::new(move |snapshot| Box::pin(run(snapshot)))),
        }
    }

    /// Attaches an ECS system used to verify the declared access footprint.
    pub fn verify_access_with<M>(mut self, verifier: impl IntoSystem<(), (), M>) -> Self {
        self.verifier = Some(Box::new(IntoSystem::into_system(verifier)));
        self
    }

    fn compile(
        mut self,
        world: &mut World,
        insertion_order: usize,
    ) -> Result<AgentSystem<T>, AgentScheduleError> {
        if matches!(self.runner, AgentRunner::Async(_)) && !self.declaration.async_allowed {
            return Err(AgentScheduleError::AsyncNotAllowed(self.name.clone()));
        }

        if self.declaration.trust_mode == AccessTrustMode::Verify {
            let verifier = self
                .verifier
                .as_mut()
                .ok_or_else(|| AgentScheduleError::MissingVerifier(self.name.clone()))?;
            let inferred = verifier.initialize(world);
            if !self.declaration.verifies(world, &inferred) {
                return Err(AgentScheduleError::AccessVerificationFailed(
                    self.name.clone(),
                ));
            }
        }

        Ok(AgentSystem {
            declaration: self.declaration,
            runner: self.runner,
            insertion_order,
        })
    }
}

/// Compile-time options for agent schedules.
#[derive(Clone, Copy, Debug, Default)]
pub struct AgentCompileOptions {
    /// Whether same-priority systems should keep insertion order.
    pub deterministic: bool,
    /// Seed reserved for future randomized tie-breaking.
    pub seed: u64,
}

/// A schedule of explicit-access agent systems sharing a snapshot extractor.
pub struct AgentSchedule<T> {
    snapshot: Box<dyn Fn(&World) -> T + Send + Sync + 'static>,
    systems: Vec<AgentSystemBuilder<T>>,
    options: AgentCompileOptions,
}

impl<T: Send + Sync + 'static> AgentSchedule<T> {
    /// Creates a new agent schedule using the given snapshot extractor.
    pub fn new(snapshot: impl Fn(&World) -> T + Send + Sync + 'static) -> Self {
        Self {
            snapshot: Box::new(snapshot),
            systems: Vec::new(),
            options: AgentCompileOptions::default(),
        }
    }

    /// Sets compile-time options for this schedule.
    pub fn with_options(mut self, options: AgentCompileOptions) -> Self {
        self.options = options;
        self
    }

    /// Adds a system to this schedule.
    pub fn add_system(&mut self, system: AgentSystemBuilder<T>) -> &mut Self {
        self.systems.push(system);
        self
    }

    /// Compiles this schedule into a runnable artifact.
    pub fn compile(
        self,
        world: &mut World,
    ) -> Result<CompiledAgentSchedule<T>, AgentScheduleError> {
        let mut systems = Vec::with_capacity(self.systems.len());
        for (insertion_order, system) in self.systems.into_iter().enumerate() {
            systems.push(system.compile(world, insertion_order)?);
        }

        if self.options.deterministic {
            systems.sort_by(|a, b| {
                b.declaration
                    .priority
                    .cmp(&a.declaration.priority)
                    .then_with(|| a.insertion_order.cmp(&b.insertion_order))
            });
        } else {
            systems.sort_by(|a, b| b.declaration.priority.cmp(&a.declaration.priority));
        }

        Ok(CompiledAgentSchedule {
            snapshot: self.snapshot,
            systems,
            options: self.options,
        })
    }
}

/// A compiled executable artifact for an [`AgentSchedule`].
pub struct CompiledAgentSchedule<T> {
    snapshot: Box<dyn Fn(&World) -> T + Send + Sync + 'static>,
    systems: Vec<AgentSystem<T>>,
    options: AgentCompileOptions,
}

impl<T: Send + Sync + 'static> CompiledAgentSchedule<T> {
    /// Returns the compile options used for this schedule.
    pub fn options(&self) -> AgentCompileOptions {
        self.options
    }

    /// Runs the compiled schedule against a single world.
    pub fn run(&mut self, world: &mut World) {
        let snapshot = WorldSnapshot {
            world_id: world.id(),
            data: Arc::new((self.snapshot)(world)),
        };

        for system in &self.systems {
            let commands = match &system.runner {
                AgentRunner::Sync(run) => {
                    let mut commands = AgentCommands::default();
                    run(&snapshot, &mut commands);
                    commands
                }
                AgentRunner::Async(run) => block_on(run(snapshot.clone())),
            };

            commands.apply(world);
        }
    }

    /// Converts this schedule into a batch executor.
    pub fn into_batch(self) -> BatchCompiledSchedule<T> {
        BatchCompiledSchedule { inner: self }
    }
}

/// A batch executor for compiled agent schedules.
pub struct BatchCompiledSchedule<T> {
    inner: CompiledAgentSchedule<T>,
}

impl<T: Send + Sync + 'static> BatchCompiledSchedule<T> {
    /// Runs this schedule over multiple worlds.
    pub fn run_batch(&mut self, worlds: &mut [World]) {
        for world in worlds {
            self.inner.run(world);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::{Res, ResMut};

    #[derive(Component)]
    struct Position;

    #[derive(Resource, Default, PartialEq, Eq, Debug)]
    struct Score(i32);

    #[test]
    fn access_declaration_verifies_against_system() {
        fn verifier(_query: crate::system::Query<&Position>, _score: Res<Score>) {}

        let mut world = World::new();
        let declaration = AccessDeclaration::new()
            .read_component::<Position>()
            .read_resource::<Score>()
            .verify();

        let builder = AgentSystemBuilder::sync(
            "verify",
            declaration,
            |_snapshot: &WorldSnapshot<()>, _commands| {},
        )
        .verify_access_with(verifier);

        let result = builder.compile(&mut world, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn compiled_agent_schedule_runs_sync_and_async_commands() {
        let mut world = World::new();
        world.insert_resource(Score::default());

        let mut schedule = AgentSchedule::new(|world: &World| world.resource::<Score>().0);
        schedule.add_system(AgentSystemBuilder::sync(
            "sync",
            AccessDeclaration::new().write_resource::<Score>().trusted(),
            |snapshot: &WorldSnapshot<i32>, commands| {
                let next = *snapshot.view() + 1;
                commands.insert_resource(Score(next));
            },
        ));
        schedule.add_system(AgentSystemBuilder::asynchronous(
            "async",
            AccessDeclaration::new()
                .write_resource::<Score>()
                .async_allowed()
                .trusted(),
            |snapshot: WorldSnapshot<i32>| async move {
                let next = *snapshot.view() + 2;
                let mut commands = AgentCommands::default();
                commands.insert_resource(Score(next));
                commands
            },
        ));

        let mut compiled = schedule.compile(&mut world).unwrap();
        compiled.run(&mut world);
        assert_eq!(world.resource::<Score>().0, 2);
    }

    #[test]
    fn batch_schedule_runs_all_worlds() {
        let mut first = World::new();
        first.insert_resource(Score(1));
        let mut second = World::new();
        second.insert_resource(Score(5));

        let mut schedule = AgentSchedule::new(|world: &World| world.resource::<Score>().0);
        schedule.add_system(AgentSystemBuilder::sync(
            "batch",
            AccessDeclaration::new().write_resource::<Score>().trusted(),
            |snapshot: &WorldSnapshot<i32>, commands| {
                commands.insert_resource(Score(*snapshot.view() + 10));
            },
        ));

        let mut batch = schedule.compile(&mut first).unwrap().into_batch();
        let mut worlds = [first, second];
        batch.run_batch(&mut worlds);

        assert_eq!(worlds[0].resource::<Score>().0, 11);
        assert_eq!(worlds[1].resource::<Score>().0, 15);
    }

    #[test]
    fn priority_order_is_deterministic() {
        let mut world = World::new();
        world.insert_resource(Score::default());

        let mut schedule = AgentSchedule::new(|_world: &World| ());
        schedule.add_system(AgentSystemBuilder::sync(
            "low",
            AccessDeclaration::new()
                .write_resource::<Score>()
                .priority(1)
                .trusted(),
            |_snapshot: &WorldSnapshot<()>, commands| {
                commands.push(|world| {
                    world.resource_mut::<Score>().0 = 1;
                });
            },
        ));
        schedule.add_system(AgentSystemBuilder::sync(
            "high",
            AccessDeclaration::new()
                .write_resource::<Score>()
                .priority(10)
                .trusted(),
            |_snapshot: &WorldSnapshot<()>, commands| {
                commands.push(|world| {
                    world.resource_mut::<Score>().0 = 10;
                });
            },
        ));

        let mut compiled = schedule
            .with_options(AgentCompileOptions {
                deterministic: true,
                seed: 7,
            })
            .compile(&mut world)
            .unwrap();
        compiled.run(&mut world);
        assert_eq!(world.resource::<Score>().0, 1);
    }

    #[test]
    fn verification_fails_when_declaration_is_too_narrow() {
        fn verifier(_score: ResMut<Score>) {}

        let mut world = World::new();
        world.insert_resource(Score::default());

        let builder = AgentSystemBuilder::sync(
            "too_narrow",
            AccessDeclaration::new().read_resource::<Score>().verify(),
            |_snapshot: &WorldSnapshot<()>, _commands| {},
        )
        .verify_access_with(verifier);

        let result = builder.compile(&mut world, 0);
        assert!(matches!(
            result,
            Err(AgentScheduleError::AccessVerificationFailed(_))
        ));
    }
}
