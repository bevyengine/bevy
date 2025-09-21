---
title: Entity Spawn Ticks
authors: ["@urben1680", "@specificprotagonist"]
pull_requests: [19047, 19350]
---

In previous versions of Bevy, keeping track of which entities have been spawned since the last time a system ran could only be done indirectly by writing your own logic.

The new `SpawnDetails` query data and `Spawned` query filter enable you to find recently spawned entities without any marker components.

## `SpawnDetails`

Use this in your query when you want to get information about the entity's spawn:

```rs
fn print_spawn_details(query: Query<(Entity, SpawnDetails)>) {
    for (entity, spawn_details) in &query {
        if spawn_details.is_spawned() {
            print!(
                "new entity {entity:?} spawned at {:?}",
                spawn_details.spawn_tick()
            );
            // if the `track_location` cargo feature is activated, this contains the source
            // code location where this entity was spawned. This has a runtime cost, so only
            // use it for debugging!
            match spawn_details.spawned_by().into_option() {
                Some(location) => println!(" by {location:?}"),
                None => println!()
            }    
        }
    }
}
```

## `Spawned`

Use this filter in your query if you are only interested in entities that were spawned after the last time your system ran:

```rust
fn system(query: Query<Entity, Spawned>) {
    for entity in &query { /* entity spawned */ }
}
```

Note that, much like `Added` and `Changed` filters, this is a "non archetypal filter", meaning it requires scanning every entity matching the query, including those that weren't spawned since the last run.
Because of this, the system above performs roughly the same as this one:

```rust
fn system(query: Query<(Entity, SpawnDetails)>) {
    for (entity, spawned) in &query {
        if spawned.is_spawned() { /* entity spawned */ }
    }
}
```

## Getter methods

You can also use helper methods on `EntityWorldMut` and `EntityCommands`:

```rust
world.entity(entity).spawn_tick()
```
