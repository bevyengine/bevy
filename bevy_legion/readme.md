# Legion

[![Build Status][build_img]][build_lnk] [![Crates.io][crates_img]][crates_lnk] [![Docs.rs][doc_img]][doc_lnk]

[build_img]: https://img.shields.io/travis/TomGillen/legion/master.svg
[build_lnk]: https://travis-ci.org/TomGillen/legion
[crates_img]: https://img.shields.io/crates/v/legion.svg
[crates_lnk]: https://crates.io/crates/legion
[doc_img]: https://docs.rs/legion/badge.svg
[doc_lnk]: https://docs.rs/legion

Legion aims to be a feature rich high performance ECS library for Rust game projects with minimal boilerplate.

## Benchmarks

Based on the [ecs_bench](https://github.com/lschmierer/ecs_bench) project.

![](bench.png)

## Getting Started

```rust
use legion::prelude::*;

// Define our entity data types
#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    dx: f32,
    dy: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Model(usize);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Static;

// Create a world to store our entities
let universe = Universe::new();
let mut world = universe.create_world();

// Create entities with `Position` and `Velocity` data
world.insert(
    (),
    (0..999).map(|_| (Position { x: 0.0, y: 0.0 }, Velocity { dx: 0.0, dy: 0.0 }))
);

// Create entities with `Position` data and a shared `Model` data, tagged as `Static`
// Shared data values are shared across many entities,
// and enable further batch processing and filtering use cases
let entities: &[Entity] = world.insert(
    (Model(5), Static),
    (0..999).map(|_| (Position { x: 0.0, y: 0.0 },))
);

// Create a query which finds all `Position` and `Velocity` components
let query = <(Write<Position>, Read<Velocity>)>::query();

// Iterate through all entities that match the query in the world
for (mut pos, vel) in query.iter(&mut world) {
    pos.x += vel.dx;
    pos.y += vel.dy;
}
```

## Features

Legion aims to be a more feature-complete game-ready ECS than many of its predecessors.

### Advanced Query Filters

The query API can do much more than pull entity data out of the world.

Additional data type filters:

```rust
// It is possible to specify that entities must contain data beyond that being fetched
let query = Read::<Position>::query()
    .filter(component::<Velocity>());
for position in query.iter(&mut world) {
    // these entities also have `Velocity`
}
```

Filter boolean operations:

```rust
// Filters can be combined with boolean operators
let query = Read::<Position>::query()
    .filter(tag::<Static>() | !component::<Velocity>());
for position in query.iter(&mut world) {
    // these entities are also either marked as `Static`, or do *not* have a `Velocity`
}
```

Filter by shared data value:

```rust
// Filters can filter by specific shared data values
let query = Read::<Position>::query()
    .filter(tag_value(&Model(3)));
for position in query.iter(&mut world) {
    // these entities all have shared data value `Model(3)`
}
```

Change detection:

```rust
// Queries can perform coarse-grained change detection, rejecting entities who's data
// has not changed since the last time the query was iterated.
let query = <(Read<Position>, Shared<Model>)>::query()
    .filter(changed::<Position>());
for (pos, model) in query.iter(&mut world) {
    // entities who have changed position
}
```

### Content Streaming

Entities can be loaded and initialized in a background `World` on separate threads and then
when ready, merged into the main `World` near instantaneously.

```rust
let universe = Universe::new();
let mut world_a = universe.create_world();
let mut world_b = universe.create_world();

// Merge all entities from `world_b` into `world_a`
// Entity IDs are guarenteed to be unique across worlds and will
// remain unchanged across the merge.
world_a.merge(world_b);
```

### Chunk Iteration

Entity data is allocated in blocks called "chunks", each approximately containing 64KiB of data. The query API exposes each chunk via `iter_chunk`. As all entities in a chunk are guarenteed to contain the same set of entity data and shared data values, it is possible to do batch processing via the chunk API.

```rust
fn render_instanced(model: &Model, transforms: &[Transform]) {
    // pass `transforms` pointer to graphics API to load into constant buffer
    // issue instanced draw call with model data and transforms
}

let query = Read::<Transform>::query()
    .filter(tag::<Model>());

for chunk in query.iter_chunks_mut(&mut world) {
    // get the chunk's model
    let model: &Model = chunk.tag().unwrap();

    // get a (runtime borrow checked) slice of transforms
    let transforms = chunk.components::<Transform>().unwrap();

    // give the model and transform slice to our renderer
    render_instanced(model, &transforms);
}
```
