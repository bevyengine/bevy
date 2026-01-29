#![expect(
    dead_code,
    reason = "Many fields are unused/unread as they are just for benchmarking purposes."
)]

use criterion::criterion_main;

mod bundles;
mod change_detection;
mod components;
mod empty_archetypes;
mod entity_cloning;
mod events;
mod fragmentation;
mod iteration;
mod observers;
mod param;
mod scheduling;
mod world;

criterion_main!(
    bundles::benches,
    change_detection::benches,
    components::benches,
    empty_archetypes::benches,
    entity_cloning::benches,
    events::benches,
    iteration::benches,
    fragmentation::benches,
    observers::benches,
    scheduling::benches,
    world::benches,
    param::benches,
);

mod world_builder {
    use bevy_ecs::world::World;
    use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};

    /// This builder generates a "hot"/realistic [`World`].
    ///
    /// Using [`World::new`] creates a "cold" world.
    /// That is, the world has a fresh entity allocator, no registered components, and generally no accumulated entropy.
    /// When a cold world is used in a benchmark, much of what is benched is registration and caching costs,
    /// and what is not benched is the cost of the accumulated entropy in world storage, entity allocators, etc.
    ///
    /// Use this in benches that are meant to reflect realistic, common, non-startup scenarios (Ex: spawn scenes, query entities, etc).
    /// Prefer [`World::new`] when creating benches for start-up costs (Ex: component registration, table creation time, etc).
    ///
    /// Note that this does have a performance cost over [`World::new`], so this should not be used in a benchmark's routine, only in its setup.
    ///
    /// Which parts of the world are sped up is highly configurable in the interest of doing the minimal work to warm up a world for a particular benchmark.
    /// (For example, despawn benches wouldn't benefit from warming up world storage.)
    pub struct WorldBuilder {
        world: World,
        rng: SmallRng,
        max_expected_entities: u32,
    }

    impl WorldBuilder {
        /// Starts the builder.
        pub fn new() -> Self {
            Self {
                world: World::new(),
                rng: SmallRng::seed_from_u64(2039482342342),
                max_expected_entities: 10_000,
            }
        }

        /// Sets the maximum expected entities that will interact with the world.
        /// By default this is `10_000`.
        pub fn with_max_expected_entities(mut self, max_expected_entities: u32) -> Self {
            self.max_expected_entities = max_expected_entities;
            self
        }

        /// Warms up the entity allocator to give out arbitrary entity ids instead of sequential ones.
        /// This also pre-allocates room in `Entities`.
        pub fn warm_up_entity_allocator(mut self) -> Self {
            // allocate
            let mut entities = Vec::new();
            entities.reserve_exact(self.max_expected_entities as usize);
            entities.extend(
                self.world
                    .entity_allocator()
                    .alloc_many(self.max_expected_entities),
            );

            // Spawn the high index to warm up `Entities`.
            let Some(high_index) = entities.last_mut() else {
                // There were no expected entities.
                return self;
            };
            self.world.spawn_empty_at(*high_index).unwrap();
            *high_index = self.world.try_despawn_no_free(*high_index).unwrap();

            // free
            entities.shuffle(&mut self.rng);
            entities
                .drain(..)
                .for_each(|e| self.world.entity_allocator_mut().free(e));

            self
        }

        /// Finishes the builder to get the warmed up world.
        pub fn build(self) -> World {
            self.world
        }
    }
}
