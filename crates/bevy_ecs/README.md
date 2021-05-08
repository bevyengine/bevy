# Bevy ECS

[![Crates.io](https://img.shields.io/crates/v/bevy_ecs.svg)](https://crates.io/crates/bevy_ecs)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/HEAD/LICENSE)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/gMUk5Ph)

## What is Bevy ECS?

Bevy ECS is the Entity Component System used in and developed for the game engine [Bevy][bevy]. Even though it was created as part of the game engine, Bevy ECS can be used standalone and in combination with other projects or game engines.

## About

Entity Component System is an architectural pattern using composition to provide greater flexibility.

### Main concepts

* Entities are identifiers for collections of components
* Components are data structures that can be attached to entities
* Systems encode a certain behaviour of the world

Entities and components are kept in a `World`. Constructing a `Schedule` with systems allows you to simulate a tick of the world. The schedule will consider dependencies between systems and run as many of them in parallel as possible. This maximises performance, while keeping the system execution safe. You can make dependencies explicit by requesting certain execution orders using `SystemLabel`s.

## Features

Bevy ECS uses Rust's type safety to represent systems as "normal" functions and components as structs. In most cases this does not require any additional effort by the user.

```rust
fn main() {
    // Create a world
    let mut world = World::new();

    // Create a schedule and a stage
    let mut schedule = Schedule::default();
    let mut update = SystemStage::parallel();

    // Add a system to the stage
    update.add_system(print_a_message.system());
    
    // Add the prepared stage to the schedule
    schedule.add_stage("update", update);

    // We will simulate 10 frames
    for iteration in 1..=10 {
        println!("Simulating frame {}/10", iteration);
        schedule.run_once(&mut world);
    }
}

// This function serves as a system
fn print_a_message() {
    println!("System is running", my_event);
}
```

### Component storage

A unique feature of Bevy ECS is the support for multiple component storage types.

* Tables: fast and cache friendly iteration, but slower adding and removing of components
* Sparse Sets: fast adding and removing of components, but slower iteration

The used storage type can be configured per component and defaults to table storage. The example [`component_storage.rs`](examples/component_storage.rs) shows how to configure the storage type for a component.

### Resources

A common pattern when working with ECS is the creation of global singleton components. Bevy ECS makes this pattern a first class citizen. `Resource`s are a special kind of component that do not belong to any entity. Bevy itself makes heavy use of Resources as a way to configure systems.

The example [`resources.rs`](examples/resources.rs) uses a resource to keep a counter that can be increased by a system and read from other systems.

### Events

Events offer a short-lived communication channel between one to many systems. Events can be sent using `EventWriter` and received with `EventReader`. Very little boilerplate is required to register a struct as an event.

A minimal set up to use events can be seen in [`events.rs`](examples/events.rs).

### Change detection

Bevy ECS includes a simple way of detecting change in resources and entities. Resources have `is_changed` and `is_added` functions and entities can be queried based on changed or added components.

The example [`change_detection.rs`](examples/change_detection.rs) shows how to query only for updated entities and react on changes in resources.

[bevy]: https://bevyengine.org/
