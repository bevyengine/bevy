---
title: "Trigger trait is no longer unsafe"
pull_requests: [25668]
---

The `Trigger` trait is no longer unsafe, but in order to accomplish that some new
associated types and functions were added:

- `Trigger::State` represents the internal state of the trigger, which is passed
  to `World::trigger` functions.
- `Trigger::View` represents a mutable view of the trigger, which is passed to
  observers as part of the `On` system input.
- `Trigger::reborrow` converts a `&'a mut State<'_>` into a `View<'a>`, allowing
  safe mutable access to the trigger's internal state.
- `Trigger::trigger` now accepts a `&mut Self::State<'_>` as its first argument,
  instead of `&mut self`.

For simple ZST `Trigger` implementations:

```rust
pub struct MyTrigger {
    // ...
}

// Bevy 0.20
unsafe impl<E: Event<Trigger = Self>> Trigger<E> for MyTrigger {
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        // ...
    }
}

// Bevy 0.21
impl<E: Event<Trigger = Self>> Trigger<E> for MyTrigger {
    type State<'input> = Self;
    type View<'input> = Self;

    fn reborrow(state: &mut Self::State<'_>) -> Self::View<'_> {
        MyTrigger
    }

    unsafe fn trigger(
        state: &mut Self::State<'_>,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        // ...
    }
}
```

For `Trigger` implementations with mutable state:

```rust
pub struct MyStatefulTrigger {
    thing: bool,
}

// Bevy 0.19
unsafe impl<E: Event<Trigger = Self>> Trigger<E> for MyStatefulTrigger {
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        // ...
    }
}

// Bevy 0.21
impl<E: Event<Trigger = Self>> Trigger<E> for MyStatefulTrigger {
    type State<'input> = Self;
    type View<'input> = &'input mut Self;

    fn reborrow(state: &mut Self::State<'_>) -> Self::View<'_> {
        state
    }

    unsafe fn trigger(
        state: &mut Self::State<'_>,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        // ...
    }
}
```

For `Trigger` implementations with internal lifetimes:

```rust
pub struct MyTrigger<'a> {
    thing: &'a mut bool,
}

// Bevy 0.19
unsafe impl<'a, E: Event<Trigger = Self>> Trigger<E> for MyTrigger<'a> {
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        // ...
    }
}

// Bevy 0.21
impl<E: Event<Trigger = Self>> Trigger<E> for MyTrigger<'_> {
    type State<'input> = MyTrigger<'input>;
    type View<'input> = MyTrigger<'input>;

    fn reborrow(state: &mut Self::State<'_>) -> Self::View<'_> {
        MyTrigger {
            thing: &mut *state.thing,
        }
    }

    unsafe fn trigger(
        state: &mut Self::State<'_>,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        // ...
    }
}
```

Additionally, `EventPatternTrigger` has been removed; existing usages should be
able to update to use `EventTriggerView` instead.
