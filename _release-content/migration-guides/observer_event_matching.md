---
title: Moved Observer events `B` Bundle generic into the event type
pull_requests: [24013]
---

The `B: Bundle` type parameter has been removed from `On<E, B>` in observer systems.
Lifecycle events (`Add`/`Insert`/`Discard`/`Remove`/`Despawn`) have been updated
to have this `B: Bundle` directly on them.

```rust
// Bevy 0.19
world.add_observer(|on: On<Add, A>| {
    // ...
});

// Bevy 0.20
world.add_observer(|on: On<Add<A>>| {
    // ...
});
```

For custom event types that previously made use of `B: Bundle`, its recommended to do the following:

```rust
// Bevy 0.19

#[derive(Event)]
pub struct Foo;

#[derive(Component)]
pub struct Bar;

world.add_observer(|on: On<Foo, Bar>| {
    // ...
});

// Bevy 0.20

#[derive(Event)]
pub struct FooEvent;

#[derive(Component)]
pub struct Bar;

pub struct Foo<B: Bundle>(PhantomData<B>);

impl<B: Bundle> EventMatcher for Foo<B> {
    type Event = FooEvent;
    type Components = B;
}

world.add_observer(|on: On<Foo<Bar>>| {
    // ...
});
```

For lifecycle observers watching dynamic components, you now need to modify
`On<Add>` to `On<Add<()>>`:

```rust
// Bevy 0.19
world.spawn(
    Observer::new(|_: On<Add>| {
        // ...
    })
    .with_component(component_id),
);

// Bevy 0.20
world.spawn(
    Observer::new(|_: On<Add<()>>| {
        // ...
    })
    .with_component(component_id),
);
```
