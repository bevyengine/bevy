use std::time::Instant;

use bevy::{prelude::*, utils::Duration};

// Sometimes systems need their own unique "local" state. Bevy's ECS provides Local<T> resources
// for this case. Local<T> resources are unique to their system and can be initialized either
// automatically (with a `FromWorld` or `Default` implementation) or manually (by calling `config`
// on the system).
// This can be useful when:
// - You won't need access to the value from other systems
// - You have multiple instances of the same system and they each need their own unique state
// - You already have a global version of a resource that you don't want to overwrite for your
// current system

#[derive(Default)]
struct RoundCount(u32);

struct TimeSinceLastReset(Instant);
impl FromWorld for TimeSinceLastReset {
    fn from_world(world: &mut World) -> Self {
        let time = world.get_resource::<Time>().unwrap();
        Self(time.startup())
    }
}

struct SpawnEntities {
    count: u32,
}

// This local parameter will be initialized from the `Default` impl of `RoundCount`.
// It is equivalent to `Local<RoundCount, local_value::Default>`.
fn default_local(mut round_count: Local<RoundCount>) {
    info!("current round: {}", round_count.0);
    round_count.0 += 1;
}

// This local parameter will be initialized from the `FromWorld` impl of `TimeSinceLastReset`.
fn from_world_local(
    mut since_last_reset: Local<TimeSinceLastReset, local_value::FromWorld>,
    query: Query<Entity>,
    mut commands: Commands,
) {
    let now = Instant::now();
    if now.duration_since(since_last_reset.0) > Duration::from_secs(1) {
        since_last_reset.0 = now;
        info!("-- Despawning {} entities", query.iter().len());
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

// This local parameter will be initialized by calling `config` on the system. If `config` is not
// called, running the system will panic.
fn need_config_local(
    to_spawn: Local<SpawnEntities, local_value::NeedConfig>,
    mut commands: Commands,
) {
    info!("Spawning {} entities", to_spawn.count);
    for _ in 0..to_spawn.count {
        commands.spawn();
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(default_local)
        .add_system(from_world_local)
        .add_system(need_config_local.config(|config| config.0 = Some(SpawnEntities { count: 3 })))
        .run();
}
