---
title: Readonly Query State Creation
authors: ["@Eagster"]
pull_requests: [18173, 22670]
---

It has long been an annoyance in Bevy that getting immutable data from a world through a query often required mutable world access.
Previously, `QueryState` required `&mut World` to be created.
This meant you had to either make a `QueryState` using `&mut World` and keep track of it somewhere, later making a readonly query, 
or use the fallible `QueryState::try_new`/`World::try_query` functions after manually registering components and other information.

This pain point is now resolved!
Today, `QueryState::new`, `World::query`, and the like all only require `&World`.
For example, it is now possible to do the following:

```rust

#[derive(Resource)]
struct MySubWorld(World);

fn read_sub_world(sub_world: Res<MySubWorld>) {
    let query = sub_world.0.query::<&MyComponent>();
    let query = query.iter(&sub_world.0);
    for entity in query.iter() {
        // TADA!
    }
}

```

Note that using `World::query` or `QueryState::new` each time results in an uncached query.
The query state and matching tables must be recalculated each time.
As a result, for queries that are running very frequently, caching the `QueryState` (through system parameters, for example) is still a good idea.
