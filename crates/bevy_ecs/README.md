# Bevy ECS

[![Crates.io](https://img.shields.io/crates/v/bevy_ecs.svg)](https://crates.io/crates/bevy_ecs)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/HEAD/LICENSE)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

## What is Bevy ECS?

Bevy ECS is an Entity Component System custom-built for the [Bevy][bevy] game engine. It aims to be simple to use, ergonomic, fast, massively parallel, opinionated, and featureful. It was created specifically for Bevy's needs, but it can easily be used as a standalone crate in other projects.

## ECS

All app logic in Bevy uses the Entity Component System paradigm, which is often shortened to ECS. ECS is a software pattern that involves breaking your program up into Entities, Components, and Systems. Entities are unique "things" that are assigned groups of Components, which are then processed using Systems.

For example, one entity might have a `Position` and `Velocity` component, whereas another entity might have a `Position` and `UI` component. You might have a movement system that runs on all entities with a Position and Velocity component.

The ECS pattern encourages clean, decoupled designs by forcing you to break up your app data and logic into its core components. It also helps make your code faster by optimizing memory access patterns and making parallelism easier.

## Concepts

Bevy ECS is Bevy's implementation of the ECS pattern. Unlike other Rust ECS implementations, which often require complex lifetimes, traits, builder patterns, or macros, Bevy ECS uses normal Rust data types for all of these concepts:

### Components

Components are normal Rust structs. They are data stored in a `World` and specific instances of Components correlate to Entities.

```rust
struct Position { x: f32, y: f32 }
```

### Worlds

Entities, Components, and Resources are stored in a `World`. Worlds, much like Rust std collections like HashSet and Vec, expose operations to insert, read, write, and remove the data they store.

```rust
let world = World::default();
```

### Entities

Entities are unique identifiers that correlate to zero or more Components.

```rust
let entity = world.spawn()
    .insert(Position { x: 0.0, y: 0.0 })
    .insert(Velocity { x: 1.0, y: 0.0 })
    .id();

let entity_ref = world.entity(entity);
let position = entity_ref.get::<Position>().unwrap();
let velocity = entity_ref.get::<Velocity>().unwrap();
```

### Systems

Systems are normal Rust functions. Thanks to the Rust type system, Bevy ECS can use function parameter types to determine what data needs to be sent to the system. It also uses this "data access" information to determine what Systems can run in parallel with each other.

```rust
fn print_position(query: Query<(Entity, &Position)>) {
    for (entity, position) in query.iter() {
        println!("Entity {:?} is at position: x {}, y {}", entity, position.x, position.y);
    }
}
```

### Resources

Apps often require unique resources, such as asset collections, renderers, audio servers, time, etc. Bevy ECS makes this pattern a first class citizen. `Resource` is a special kind of component that does not belong to any entity. Instead, it is identified uniquely by its type:

```rust
#[derive(Default)]
struct Time {
    seconds: f32,
}

world.insert_resource(Time::default());

let time = world.get_resource::<Time>().unwrap();

// You can also access resources from Systems
fn print_time(time: Res<Time>) {
    println!("{}", time.seconds);
}
```

The [`resources.rs`](examples/resources.rs) example illustrates how to read and write a Counter resource from Systems.

### Schedules

Schedules consist of zero or more Stages, which run a set of Systems according to some execution strategy. Bevy ECS provides a few built in Stage implementations (ex: parallel, serial), but you can also implement your own! Schedules run Stages one-by-one in an order defined by the user.

The built in "parallel stage" considers dependencies between systems and (by default) run as many of them in parallel as possible. This maximizes performance, while keeping the system execution safe. You can also define explicit dependencies between systems.

## Using Bevy ECS

Bevy ECS should feel very natural for those familiar with Rust syntax:

```rust
use bevy_ecs::prelude::*;

struct Velocity {
    x: f32,
    y: f32,
}

struct Position {
    x: f32,
    y: f32,
}

// This system moves each entity with a Position and Velocity component
fn movement(query: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in query.iter_mut() {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

fn main() {
    // Create a new empty World to hold our Entities and Components
    let mut world = World::new();

    // Spawn an entity with Position and Velocity components
    world.spawn()
        .insert(Position { x: 0.0, y: 0.0 })
        .insert(Velocity { x: 1.0, y: 0.0 });

    // Create a new Schedule, which defines an execution strategy for Systems
    let mut schedule = Schedule::default();

    // Add a Stage to our schedule. Each Stage in a schedule runs all of its systems
    // before moving on to the next Stage
    schedule.add_stage("update", SystemStage::parallel()
        .with_system(movement)
    );

    // Run the schedule once. If your app has a "loop", you would run this once per loop
    schedule.run(&mut world);
}
```

## Features

### Query Filters

```rust
// Gets the Position component of all Entities with Player component and without the RedTeam component
fn system(query: Query<&Position, (With<Player>, Without<RedTeam>)>) {
    for position in query.iter() {
    }
}
```

### Change Detection

Bevy ECS tracks _all_ changes to Components and Resources.

Queries can filter for changed Components:

```rust
// Gets the Position component of all Entities whose Velocity has changed since the last run of the System
fn system(query: Query<&Position, Changed<Velocity>>) {
    for position in query.iter() {
    }
}

// Gets the Position component of all Entities that had a Velocity component added since the last run of the System
fn system(query: Query<&Position, Added<Velocity>>) {
    for position in query.iter() {
    }
}
```

Resources also expose change state:

```rust
// Prints "time changed!" if the Time resource has changed since the last run of the System
fn system(time: Res<Time>) {
    if time.is_changed() {
        println!("time changed!");
    }
}
```

The [`change_detection.rs`](examples/change_detection.rs) example shows how to query only for updated entities and react on changes in resources.

### Component Storage

Bevy ECS supports multiple component storage types.

Components can be stored in:

* **Tables**: Fast and cache friendly iteration, but slower adding and removing of components. This is the default storage type.
* **Sparse Sets**: Fast adding and removing of components, but slower iteration.

Component storage types are configurable, and they default to table storage if the storage is not manually defined.

```rs
#[derive(Component)]
struct TableStoredComponent;

#[derive(Component)]
#[component(storage = "SparseSet")]
struct SparseStoredComponent;
```

### Component Bundles

Define sets of Components that should be added together.

```rust
#[derive(Bundle, Default)]
struct PlayerBundle {
    player: Player,
    position: Position,
    velocity: Velocity,
}

// Spawn a new entity and insert the default PlayerBundle
world.spawn().insert_bundle(PlayerBundle::default());

// Bundles play well with Rust's struct update syntax
world.spawn().insert_bundle(PlayerBundle {
    position: Position { x: 1.0, y: 1.0 },
    ..Default::default()
});
```

### Events

Events offer a communication channel between one or more systems. Events can be sent using the system parameter `EventWriter` and received with `EventReader`.

```rust
struct MyEvent {
    message: String,
}

fn writer(mut writer: EventWriter<MyEvent>) {
    writer.send(MyEvent {
        message: "hello!".to_string(),
    });
}

fn reader(mut reader: EventReader<MyEvent>) {
    for event in reader.iter() {
    }
}
```

A minimal set up using events can be seen in [`events.rs`](examples/events.rs).

[bevy]: https://bevyengine.org/
