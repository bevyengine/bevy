use crate::{archetype::ArchetypeGeneration, schedule::ParallelSystemContainer, world::World};
use downcast_rs::{impl_downcast, Downcast};

pub trait ParallelSystemExecutor: Downcast + Send + Sync {
    /// Called by `SystemStage` whenever `systems` have been changed.
    fn rebuild_cached_data(&mut self, systems: &[ParallelSystemContainer]);

    fn run_systems(&mut self, systems: &mut [ParallelSystemContainer], world: &mut World);
}

impl_downcast!(ParallelSystemExecutor);

pub struct SingleThreadedExecutor {
    /// Last archetypes generation observed by parallel systems.
    archetype_generation: ArchetypeGeneration,
}

impl Default for SingleThreadedExecutor {
    fn default() -> Self {
        Self {
            // MAX ensures access information will be initialized on first run.
            archetype_generation: ArchetypeGeneration::new(usize::MAX),
        }
    }
}
impl ParallelSystemExecutor for SingleThreadedExecutor {
    fn rebuild_cached_data(&mut self, _: &[ParallelSystemContainer]) {}

    fn run_systems(&mut self, systems: &mut [ParallelSystemContainer], world: &mut World) {
        self.update_archetypes(systems, world);

        for system in systems {
            if system.should_run() {
                system.system_mut().run((), world);
            }
        }
    }
}

impl SingleThreadedExecutor {
    /// Calls system.new_archetype() for each archetype added since the last call to
    /// [update_archetypes] and updates cached archetype_component_access.
    fn update_archetypes(&mut self, systems: &mut [ParallelSystemContainer], world: &World) {
        let archetypes = world.archetypes();
        let old_generation = self.archetype_generation;
        let new_generation = archetypes.generation();
        if old_generation == new_generation {
            return;
        }

        let archetype_index_range = if old_generation.value() == usize::MAX {
            0..archetypes.len()
        } else {
            old_generation.value()..archetypes.len()
        };
        for archetype in archetypes.archetypes[archetype_index_range].iter() {
            for container in systems.iter_mut() {
                let system = container.system_mut();
                system.new_archetype(archetype);
            }
        }

        self.archetype_generation = new_generation;
    }
}
