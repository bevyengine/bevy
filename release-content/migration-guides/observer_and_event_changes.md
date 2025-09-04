---
title: Observer / Event API Changes
pull_requests: [20731, 19440, 19596]
---

The observer "trigger" API has changed a bit to improve clarity and type-safety.

```rust
// Old
commands.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Spawned player {}", trigger.target());
});

// New
commands.add_observer(|add: On<Add, Player>| {
    info!("Spawned player {}", add.entity);
});
```

The `Trigger` type used inside observers has been renamed to `On` to encourage developers to think about this parameter _as_ the event. We also recommend naming the variable after the event type (ex: `add`).

To reduce repetition and improve readability, the `OnAdd`, `OnInsert`, `OnReplace`, `OnRemove`, and `OnDespawn`
observer events have also been renamed to `Add`, `Insert`, `Replace`, `Remove`, and `Despawn` respectively.
In rare cases where the `Add` event conflicts with the `std::ops::Add` trait, you may need to disambiguate,
for example by using `ops::Add` for the trait. We encourage removing the "On" from custom events named `OnX`.

Types implementing `Event` can no longer be triggered from _all contexts. By default `Event` is a "global" / "target-less" event.

Events that target an entity should now derive `EntityEvent`, and they will now store the target entity _on_ the event type, which is accessible via `EntityEvent::event_target`. Additionally, `world.trigger_targets` has been removed in favor of a single `world.trigger` API:

```rust
// Old
#[derive(Event)]
struct Explode;

world.trigger_targets(Explode, entity);

// New
#[derive(EntityEvent)]
struct Explode {
    entity: Entity
}

world.trigger(Explode { entity });
```

Triggering an entity event for multiple entities now requires multiple calls to `trigger`:

```rust
// Old
world.trigger_targets(Explode, [e1, e2]);

// New - Variant 1
world.trigger(Explode { entity: e1 });
world.trigger(Explode { entity: e2 });

// New - Variant 2
for entity in [e1, e2] {
    world.trigger(Explode { entity });
}
```

`On::target()` no longer exists for all event types. Instead, you should prefer accessing the "target entity" field on the events that target entities:

```rust
// Old
commands.add_observer(|trigger: Trigger<Explode>| {
    info!("{} exploded!", trigger.target());
});

// New
commands.add_observer(|explode: On<Explode>| {
    info!("{} exploded!", explode.entity);
    // you can also use `EntityEvent::event_target`, but we encourage
    // using direct field access when possible, for better documentation and clarity.
    info!("{} exploded!", explode.event_target());
});
```

"Propagation functions", such as `On::propagate` are now _only_ available on `On<E>` when `E: EntityEvent<Trigger = PropagateEntityTrigger>`.

Enabling propagation is now down using, which defaults to `ChildOf` propagation:

```rust
#[derive(EntityEvent)]
#[entity_event(propagate)]
struct Click {
    entity: Entity,
}
```

Setting a custom propagation `Traversal` implementation now uses `propagate` instead of `traversal`:

```rust
// OLd
#[derive(Event)]
#[event(traversal = &'static ChildOf)]
struct Click;

// New
#[derive(EntityEvent)]
#[entity_event(propagate = &'static ChildOf)]
struct Click {
    entity: Entity,
}
```

Animation events (used in `AnimationPlayer`) must now derive `AnimationEvent`. Accessing the animation player entity is now done via the `trigger()`.

```rust
// Old
#[derive(Event)]
struct SayMessage(String);

animation.add_event(0.2, SayMessage("hello".to_string()));
world.entity_mut(animation_player).observe(|trigger: Trigger<SayMessage>| {
    println!("played on", trigger.target());
})

// New
#[derive(AnimationEvent)]
struct SayMessage(String);

animation.add_event(0.2, SayMessage("hello".to_string()));
world.entity_mut(animation_player).observe(|say_message: On<SayMessage>| {
    println!("played on", say_message.trigger().animation_player);
})
```

For "component lifecycle events", accessing _all_ of the components that triggered the event has changed:

```rust
// Old
commands.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("{}", trigger.components());
});

// New
commands.add_observer(|add: On<Add, Player>| {
    info!("{}", add.trigger().components);
});
```
