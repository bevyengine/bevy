# Bevy ECS

[![Crates.io](https://img.shields.io/crates/v/bevy_ecs.svg)](https://crates.io/crates/bevy_ecs)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/HEAD/LICENSE)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/gMUk5Ph)

## What is Bevy ECS?

Bevy ECS is the Entity Component System used in and developed for the game engine [Bevy][bevy]. Even though it was created as part of the game engine, Bevy ECS can be used standalone and in combination with other projects or game engines.

## About

Entity component system is an architectural pattern using composition to provide greater flexibility.

### Main concepts

* Entities are identifiers for collections of components
* Components are data structures that can be attached to entities
* Systems encode a certain behaviour of the world

Entities and components are kept in a `World`. Constructing a `Schedule` with systems allows you to simulate a tick of the world. The schedule will consider dependencies between systems and run as many of them in parallel as possible. This maximises performance, while keeping the system execution safe. You can make dependencies explicit by requesting certain execution orders using `SystemLabel`s.

## Features

Bevy ECS uses Rust's type safety to represent systems as "normal" functions and components as structs. In most cases this does not require any additional effort by the user.

### Component storage

A unique feature of Bevy ECS is the support for multiple component storage types.

* Tables: fast and cache friendly iteration, but slower adding and removing of components
* Sparse Sets: fast adding and removing of components, but slower iteration

The used storage type can be configured per component and defaults to table storage.

### Resources

A common pattern when working with ECS is the creation of global singleton components. Bevy ECS makes this pattern a first class citizen. `Resource`s are a special kind of component that do not belong to any entity. Bevy makes heavy use of Resources as a way to configure systems.

### Events

Events offer a short-lived communication channel between one to many systems.

To prepare a world for events of type `MyEvent`, the event needs to be registered as a resource. A system managing the event can then be added to your schedule:

```rust
// this is our event
struct MyEvent {
    pub message: String,
}

// Create a world and add the event as a resource
let mut world = World::new();
world.insert_resource(Events::<MyEvent>::default());

// Create a schedule and a stage
let mut schedule = Schedule::default();
let mut update = SystemStage::parallel();

// Add the event managing system to the stage and register the stage in with the schedule
update.add_system(Events::<MyEvent>::update_system.system());
schedule.add_stage("update", update);
```

After preparing your world like above, you can send and receive events in other systems like so:

```rust
fn sending_system(
    mut event_writer: EventWriter<MyEvent>,
) {
    event_writer.send(MyEvent {
        message: "MyEvent just happened!".to_string(),
    });
}

fn recieving_system(mut event_reader: EventReader<MyEvent>) {
    for my_event in event_reader.iter() {
        println!("{}", my_event.message);
    }
}
```

[bevy]: https://bevyengine.org/
