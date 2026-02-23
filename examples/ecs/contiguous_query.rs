//! Demonstrates how contiguous queries work.
//!
//! Contiguous iteration enables getting slices of contiguously lying components (which lie in the same table), which for example
//! may be used for simd-operations, which may accelerate an algorithm.
//!
//! Contiguous iteration may be used for example via [`Query::contiguous_iter`], [`Query::contiguous_iter_mut`],
//! both of which return an option which is only [`None`] when the query doesn't support contiguous
//! iteration due to it not being dense (iteration happens on archetypes, not tables) or filters not being archetypal.
//!
//! For further documentation refer to:
//! - [`Query::contiguous_iter`]
//! - [`ContiguousQueryData`](`bevy::ecs::query::ContiguousQueryData`)
//! - [`ArchetypeFilter`](`bevy::ecs::query::ArchetypeFilter`)

use bevy::prelude::*;

#[derive(Component)]
/// When the value reaches 0.0 the entity dies
pub struct Health(pub f32);

#[derive(Component)]
/// Each tick an entity will have it's health multiplied by the factor, which
/// for a big amount of entities can be accelerated using contiguous queries
pub struct HealthDecay(pub f32);

fn apply_health_decay(mut query: Query<(&mut Health, &HealthDecay)>) {
    // contiguous_iter_mut() would return None if query couldn't be iterated contiguously
    for (mut health, decay) in query.contiguous_iter_mut().unwrap() {
        // all data slices returned by component queries are the same size
        assert!(health.len() == decay.len());
        // we could also bypass change detection via bypass_change_detection() because we do not
        // use it anyways.
        for (health, decay) in health.iter_mut().zip(decay) {
            health.0 *= decay.0;
        }
    }
}

fn finish_off_first(mut commands: Commands, mut query: Query<(Entity, &mut Health)>) {
    if let Some((entity, mut health)) = query.iter_mut().next() {
        health.0 -= 1.0;
        if health.0 <= 0.0 {
            commands.entity(entity).despawn();
            println!("Finishing off {entity:?}");
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, (apply_health_decay, finish_off_first).chain())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let mut i = 0;
    commands.spawn_batch(std::iter::from_fn(move || {
        i += 1;
        if i == 10_000 {
            None
        } else {
            Some((Health(i as f32 * 5.0), HealthDecay(0.9)))
        }
    }));
}
