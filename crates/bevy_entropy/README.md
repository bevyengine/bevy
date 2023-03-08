# Bevy Entropy

[![Crates.io](https://img.shields.io/crates/v/bevy_entropy.svg)](https://crates.io/crates/bevy_entropy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/HEAD/LICENSE)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

## What is Bevy Entropy?

Bevy Entropy is a plugin to provide integration of `rand` ecosystem PRNGs in an ECS friendly way. It provides a set of wrapper component and resource types that allow for safe access to a PRNG for generating random numbers, giving features like reflection, serialization for free. And with these types, it becomes possible to have determinism with the usage of these integrated PRNGs in ways that work with multi-threading and also avoid pitfalls such as unstable query iteration order.

## Prerequisites

For a PRNG crate to be usable with Bevy Entropy, at its minimum, it must implement `RngCore` and `SeedableRng` traits from `rand_core`, as well as `PartialEq`, `Clone`, and `Debug` traits. For reflection/serialization support, it should also implement `Serialize`/`Deserialize` traits from `serde`, though this can be disabled if one is not making use of reflection/serialization. As long as these traits are implemented, the PRNG can just be plugged in without an issue.

## Concepts

Bevy Entropy operates around a global entropy source provided as a resource, and then entropy components that can then be attached to entities. The use of resources/components allow the ECS to schedule systems appropriately so to make it easier to achieve determinism.

### GlobalEntropy

`GlobalEntropy` is the main resource for providing a global entropy source. It can only be accessed via a `ResMut` if looking to generate random numbers from it, as `RngCore` only exposes `&mut self` methods. As a result, working with `ResMut<GlobalEntropy<T>>` means any systems that access it will not be able to run in parallel to each other, as the `mut` access requires the scheduler to ensure that only one system at a time is accessing it. Therefore, if one intends on parallelising RNG workloads, limiting use/access of `GlobalEntropy` is vital. However, if one intends on having a single seed to deterministic control/derive many RNGs, `GlobalEntropy` is the best source for this purpose.

### EntropyComponent

`EntropyComponent` is a wrapper component that allows for entities to have their own RNG source. In order to generate random numbers from it, the `EntropyComponent` must be accessed with a `&mut` reference. Doing so will limit systems accessing the same source, but to increase parallelism, one can create many different sources instead. For ensuring determinism, query iteration must be accounted for as well as it isn't stable. Therefore, entities that need to perform some randomised task should 'own' their own `EntropyComponent`.

`EntropyComponent` can be seeded directly, or be created from a `GlobalEntropy` source or other `EntropyComponent`s.

### Forking

If cloning creates a second instance that shares the same state as the original, forking derives a new state from the original, leaving the original 'changed' and the new instance with a randomised seed. Forking RNG instances from a global source is a way to ensure that one seed produces many random yet deterministic states, making it difficult to predict outputs from many sources and also ensuring no one source shares the same state either with the original or with each other.

Bevy Entropy approaches forking via `From` implementations of the various component/resource types, making it straightforward to use.

## Using Bevy Entropy

Usage of Bevy Entropy can range from very simple to quite complex use-cases, all depending on whether one cares about deterministic output or not.

### Basic Usage

At the simplest case, using `GlobalEntropy` directly for all random number generation, though this does limit how well systems using `GlobalEntropy` can be parallelised. All systems that access `GlobalEntropy` will run serially to each other.

```rust
use bevy_ecs::prelude::ResMut;
use bevy_entropy::prelude::*;
use rand_core::RngCore;
use rand_chacha::ChaCha8Rng;

fn print_random_value(mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
    println!("Random value: {}", rng.next_u32());
}
```

### Forking RNGs

For seeding `EntropyComponent`s from a global source, it is best to make use of forking instead of generating the seed value directly.

```rust
use bevy_ecs::{
    prelude::{Component, ResMut},
    system::Commands,
};
use bevy_entropy::prelude::*;
use rand_chacha::ChaCha8Rng;

#[derive(Component)]
struct Source;

fn setup_source(mut commands: Commands, mut global: ResMut<GlobalEntropy<ChaCha8Rng>>) {
    commands
        .spawn((
            Source,
            EntropyComponent::from(&mut global),
        ));
}
```

`EntropyComponent`s can be seeded/forked from other `EntropyComponent`s as well.

```rust
use bevy_ecs::{
    prelude::{Component, Query, With, Without},
    system::Commands,
};
use bevy_entropy::prelude::*;
use rand_chacha::ChaCha8Rng;

#[derive(Component)]
struct Npc;

#[derive(Component)]
struct Source;

fn setup_npc_from_source(
   mut commands: Commands,
   mut q_source: Query<&mut EntropyComponent<ChaCha8Rng>, (With<Source>, Without<Npc>)>,
) {
   let mut source = q_source.single_mut();
   for _ in 0..2 {
       commands
           .spawn((
               Npc,
               EntropyComponent::from(&mut source)
           ));
   }
}
```

### Enabling Determinism

Determinism relies on not just how RNGs are seeded, but also how systems are grouped and ordered relative to each other. Systems accessing the same source/entities will run serially to each other, but if you can separate entities into different groups that do not overlap with each other, systems can then run in parallel as well. Overall, care must be taken with regards to system ordering and scheduling, as well as unstable query iteration meaning the order of entities a query iterates through is not the same per run. This can affect the outcome/state of the PRNGs, producing different results.

The examples provided in this repo demonstrate the two different concepts of parallelisation and deterministic outputs, so check them out to see how one might achieve determinism.
