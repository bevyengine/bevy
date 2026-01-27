---
title: Readonly Query State Creation
authors: ["@Eagster"]
pull_requests: [18173, 22670]
---

It has long been an annoyance in Bevy that getting immutable data from a world through a query often required mutable world access.
Previously, `QueryState` required `&mut World` to be created.
This meant you had to either make a `QueryState` using `&mut World` and keep track of it somewhere,
or use the fallible `QueryState::try_new`/`World::try_query` functions after manually registering components.

This pain point is now resolved!
Today, `QueryState::new`, `World::query`, and the like all only require `&World`.
This will cause some breaking changes (see the migration guide for those), but it is well worth it for the possibilities it opens up.
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

Note that using `World::query` or `QueryState::new` initializes a new query cache each time.
The query state and matching tables must be recalculated each time.
As a result, for queries that are running very frequently, caching the `QueryState` (through system parameters, for example) is still a good idea.
