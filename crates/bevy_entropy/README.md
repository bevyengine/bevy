# Bevy Entropy

[![Crates.io](https://img.shields.io/crates/v/bevy_entropy.svg)](https://crates.io/crates/bevy_entropy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/HEAD/LICENSE)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

## What is Bevy Entropy?

Bevy Entropy is a plugin to provide integration of `rand` ecosystem PRNGs in an ECS friendly way. It provides a set of wrapper component and resource types that allow for safe access to a PRNG for generating random numbers, giving features like reflection, serialization for free. And with these types, it becomes possible to have determinism with the usage of these integrated PRNGs in ways that work with multi-threading and also avoid pitfalls such as unstable query iteration order.

## Prerequisites

For a PRNG crate to be usable with Bevy Entropy, at its minimum, it must implement `RngCore` and `SeedableRng` traits from `rand_core`, as well as `PartialEq`, `Clone`, and `Debug` traits. For reflection/serialization support, it should also implement `Serialize`/`Deserialize` traits from `serde`, though this can be disabled if one is not making use of reflection/serialization. As long as these traits are implemented, the PRNG can just be plugged in without an issue.

## Overview

Games often use randomness as a core mechanic. For example, card games generate a random deck for each game and killing monsters in an RPG often rewards players with a random item. While randomness makes games more interesting and increases replayability, it also makes games harder to test and prevents advanced techniques such as [deterministic lockstep](https://gafferongames.com/post/deterministic_lockstep/).

Let's pretend you are creating a poker game where a human player can play against the computer. The computer's poker logic is very simple: when the computer has a good hand, it bets all of its money. To make sure the behavior works, you write a test to first check the computer's hand and if it is good confirm that all its money is bet. If the test passes does it ensure the computer behaves as intended? Sadly, no.

Because the deck is randomly shuffled for each game (without doing so the player would already know the card order from the previous game), it is not guaranteed that the computer player gets a good hand and thus the betting logic goes unchecked.
While there are ways around this (a fake deck that is not shuffled, running the test many times to increase confidence, breaking the logic into units and testing those) it would be very helpful to have randomness as well as a way to make it _less_ random.

Luckily, when a computer needs a random number it doesn't use real randomness and instead uses a [pseudorandom number generator](https://en.wikipedia.org/wiki/Pseudorandom_number_generator). Popular Rust libraries containing pseudorandom number generators are [`rand`](https://crates.io/crates/rand) and [`fastrand`](https://crates.io/crates/fastrand).

Pseudorandom number generators require a source of [entropy](https://en.wikipedia.org/wiki/Entropy) called a [random seed](https://en.wikipedia.org/wiki/Random_seed). The random seed is used as input to generate numbers that _appear_ random but are instead in a specific and deterministic order. For the same random seed, a pseudorandom number generator always returns the same numbers in the same order.

For example, let's say you seed a pseudorandom number generator with `1234`.
You then ask for a random number between `10` and `99` and the pseudorandom number generator returns `12`.
If you run the program again with the same seed (`1234`) and ask for another random number between `1` and `99`, you will again get `12`.
If you then change the seed to `4567` and run the program, more than likely the result will not be `12` and will instead be a different number.
If you run the program again with the `4567` seed, you should see the same number from the previous `4567`-seeded run.

There are many types of pseudorandom number generators each with their own strengths and weaknesses. Because of this, Bevy does not include a pseudorandom number generator. Instead, the `bevy_entropy` plugin includes a source of entropy to use as a random seed for your chosen pseudorandom number generator.

Note that Bevy currently has [other sources of non-determinism](https://github.com/bevyengine/bevy/discussions/2480) unrelated to pseudorandom number generators.

## Concepts

Bevy Entropy operates around a global entropy source provided as a resource, and then entropy components that can then be attached to entities. The use of resources/components allow the ECS to schedule systems appropriately so to make it easier to achieve determinism.

### GlobalEntropy

`GlobalEntropy` is the main resource for providing a global entropy source. It can only be accessed via a `ResMut` if looking to generate random numbers from it, as `RngCore` only exposes `&mut self` methods. As a result, working with `ResMut<GlobalEntropy<T>>` means any systems that access it will not be able to run in parallel to each other, as the `mut` access requires the scheduler to ensure that only one system at a time is accessing it. Therefore, if one intends on parallelising RNG workloads, limiting use/access of `GlobalEntropy` is vital. However, if one intends on having a single seed to deterministic control/derive many RNGs, `GlobalEntropy` is the best source for this purpose.

### EntropyComponent

`EntropyComponent` is a wrapper component that allows for entities to have their own RNG source. In order to generate random numbers from it, the `EntropyComponent` must be accessed with a `&mut` reference. Doing so will limit systems accessing the same source, but to increase parallelism, one can create many different sources instead. For ensuring determinism, query iteration must be accounted for as well as it isn't stable. Therefore, entities that need to perform some randomised task should 'own' their own `EntropyComponent`.

`EntropyComponent` can be seeded directly, or be created from a `GlobalEntropy` source or other `EntropyComponent`s.

### Forking

If cloning creates a second instance that shares the same state as the original, forking derives a new state from the original, leaving the original 'changed' and the new instance with a randomised seed. Forking RNG instances from a global source is a way to ensure that one seed produces many deterministic states, while making it difficult to predict outputs from many sources and also ensuring no one source shares the same state either with the original or with each other.

Bevy Entropy approaches forking via `From` implementations of the various component/resource types, making it straightforward to use.

## Using Bevy Entropy

Usage of Bevy Entropy can range from very simple to quite complex use-cases, all depending on whether one cares about deterministic output or not.

### Registering a PRNG for use with Bevy Entropy

Before a PRNG can be used via `GlobalEntropy` or `EntropyComponent`, it must be registered via the plugin.

```rust
use bevy_app::App;
use bevy_entropy::prelude::*;
use rand_core::RngCore;
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugin(EntropyPlugin::<ChaCha8Rng>::default())
        .run();
}
```

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
