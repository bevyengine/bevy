---
title: New QueryData Types
authors: ["@ecoskey"]
pull_requests: [19602]
---

Bevy queries have some new powers for advanced users. Namely, custom `WorldQuery`
implementations can store and apply "deferred" mutations, just like `Commands`!
This release includes a few new types making use of this capability, and
we're sure third-party crates will find all kinds of new ways to do cool stuff
with this.

## `DeferredMut`

When working with immutable components in Bevy, the acts of reading and writing
component values are very clearly separated. This can be valuable, especially
if a component has expensive hooks or observers attached and `insert`ing it
has a significant cost, but in some cases it can feel like boilerplate.

`DeferredMut` is meant to improve the ergonomics of the latter case, by providing
"fake" mutable access to any component, even immutable ones! Internally, it
keeps track of any modifications and inserts them into the ECS at the next
sync point.

```rs
// without `DeferredMut`
pub fn tick_poison(
    mut commands: Commands,
    query: Query<(Entity, &Health), With<Poisoned>>
) {
    for (entity, Health(health_points)) in query {
        commands.insert(entity, Health(health_points - 1))
    }
}

// with `DeferredMut`
pub fn tick_poison(
    mut health_query: Query<DeferredMut<Health>, With<Poisoned>>
) {
    for mut health in &health_query {
         health.0 -= 1;
    }
}
   
```
