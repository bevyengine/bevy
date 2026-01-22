//! Demonstrates how contiguous queries work

use bevy::prelude::*;

#[derive(Component)]
/// When the value reaches 0.0 the entity dies
pub struct Health(pub f32);

#[derive(Component)]
/// Each tick an entity will have it's health multiplied by the factor, which
/// for a big amount of entities can be accelerated using contiguous queries
pub struct HealthDecay(pub f32);

fn apply_health_decay(mut query: Query<(&mut Health, &HealthDecay)>) {
    // as_contiguous_iter() would return None if query couldn't be iterated contiguously
    for (mut health, decay) in query.contiguous_iter_mut().unwrap() {
        // all data slices returned by component queries are the same size
        assert!(health.data_slice().len() == decay.len());
        for (health, decay) in health.data_slice_mut().iter_mut().zip(decay) {
            health.0 *= decay.0;
        }
        // we could have updated health's ticks but it is unnecessary hence we can make less work
        // health.mark_all_as_updated();
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
