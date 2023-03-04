#![allow(clippy::type_complexity)]

use bevy_app::App;
use bevy_ecs::{
    prelude::{Component, ResMut},
    query::With,
    system::{Commands, Query},
};
use bevy_entropy::prelude::*;
use rand::prelude::Rng;
use rand_chacha::ChaCha8Rng;

#[derive(Component)]
struct SourceA;

#[derive(Component)]
struct SourceB;

#[derive(Component)]
struct SourceC;

#[derive(Component)]
struct SourceD;

/// Entities having their own sources side-steps issues with parallel execution and scheduling
/// not ensuring that certain systems run before others. With an entity having its own RNG source,
/// no matter when the systems that query that entity run, it will always result in a deterministic
/// output. The order of execution will not just the RNG output, as long as the entities are
/// seeded deterministically and any systems that query a specific entity or group of entities are
/// assured to be in order.
fn main() {
    App::new()
        .add_plugin(EntropyPlugin::<ChaCha8Rng>::new().with_seed([2; 32]))
        .add_startup_system(setup_sources)
        .add_system(random_output_a)
        .add_system(random_output_b)
        .add_system(random_output_c)
        .add_system(random_output_d)
        .run();
}

fn random_output_a(mut q_source: Query<&mut EntropyComponent<ChaCha8Rng>, With<SourceA>>) {
    let mut rng = q_source.single_mut();

    println!("SourceA result: {}", rng.gen::<u32>());
}

fn random_output_b(mut q_source: Query<&mut EntropyComponent<ChaCha8Rng>, With<SourceB>>) {
    let mut rng = q_source.single_mut();

    println!("SourceB result: {}", rng.gen_bool(0.5));
}

fn random_output_c(mut q_source: Query<&mut EntropyComponent<ChaCha8Rng>, With<SourceC>>) {
    let mut rng = q_source.single_mut();

    println!("SourceC result: {}", rng.gen_range(0u32..=20u32));
}

fn random_output_d(mut q_source: Query<&mut EntropyComponent<ChaCha8Rng>, With<SourceD>>) {
    let mut rng = q_source.single_mut();

    println!("SourceD result: {:?}", rng.gen::<(u16, u16)>());
}

fn setup_sources(mut commands: Commands, mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
    commands.spawn((SourceA, EntropyComponent::from(&mut rng)));

    commands.spawn((SourceB, EntropyComponent::from(&mut rng)));

    commands.spawn((SourceC, EntropyComponent::from(&mut rng)));

    commands.spawn((SourceD, EntropyComponent::from(&mut rng)));
}
