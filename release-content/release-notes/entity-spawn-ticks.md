---
title: Entity Spawn Ticks
authors: ["@urben1680", "@specificprotagonist"]
pull_requests: [19047, 19350]
---

Keeping track which entities have been spawned since the last time a system ran could only be done indirectly by inserting marker components and do your logic on entities that match an `Added<MyMarker>` filter or in `MyMarker`'s `on_add` hook.

This has the issue however: add events react to component insertions on existing entities too. Sometimes you cannot even add your marker because the spawn call is hidden in some non-public API.

The new `SpawnDetails` query data and `Spawned` query filter enable you to find recently spawned entities without any marker components.

## `SpawnDetails`

Use this in your query when you want to get information about the entity's spawn. You might want to do that for debug purposes, using the struct's `Debug` implementation.

You can also get specific information via methods. The following example prints the entity id (prefixed with "new" if it showed up for the first time), the `Tick` it spawned at and, if the `track_location` feature is activated, the source code location where it was spawned. Said feature is not enabled by default because it comes with a runtime cost.

```rs
fn print_spawn_details(query: Query<(Entity, SpawnDetails)>) {
    for (entity, spawn_details) in &query {
        if spawn_details.is_spawned() {
            print!("new ");
        }
        print!(
            "entity {entity:?} spawned at {:?}",
            spawn_details.spawned_at()
        );
        match spawn_details.spawned_by().into_option() {
            Some(location) => println!(" by {location:?}"),
            None => println!()
        }    
    }
}
```

## `Spawned`

Use this filter in your query if you are only interested in entities that were spawned after the last time your system ran.

Note that this, like `Added<T>` and `Changed<T>`, is a non-archetypal filter. This means that your query could still go through millions of entities without yielding any recently spawned ones. Unlike filters like `With<T>` which can easily skip all entities that do not have `T` without checking them one-by-one.

Because of this, these systems have roughly the same performance:

```rs
fn system1(query: Query<Entity, Spawned>) {
    for entity in &query { /* entity spawned */ }
}

fn system2(query: Query<(Entity, SpawnDetails)>) {
    for (entity, spawned) in &query {
        if spawned.is_spawned() { /* entity spawned */ }
    }
}
```

## Getter methods

Getting around this weakness of non-archetypal filters can be to check only specific entities for their spawn tick: The method `spawned_at` was added to all entity pointer structs, such as `EntityRef`, `EntityMut` and `EntityWorldMut`.

In this example we want to filter for entities that were spawned after a certain `tick`:

```rs
fn filter_spawned_after(
    entities: impl IntoIterator<Item = Entity>,
    world: &World,
    tick: Tick,
) -> impl Iterator<Item = Entity> {
    let now = world.last_change_tick();
    entities.into_iter().filter(move |entity| world
        .entity(*entity)
        .spawned_at()
        .is_newer_than(tick, now)
    )
}
```

---

The tick is stored in `Entities`. It's method `entity_get_spawned_or_despawned_at` not only returns when a living entity spawned at, it also returns when a despawned entity found it's bitter end.

Note however that despawned entities can be replaced by Bevy at any following spawn. Then this method returns `None` for the despawned entity. The same is true if the entity is not even spawned yet, only allocated.
