---
title: Event / Observer Overhaul
authors: ["@cart", "@Jondolf", "@alice-i-cecile", "@hukasu", "@oscar-benderstone", "@Zeophlite", "@gwafotapa"]
pull_requests: [20731, 19596, 19663, 19611, 19935, 20274]
---

Bevy's Observer API landed a few releases ago, and it has quickly become one of our most popular features. In **Bevy 0.17** we rearchitected and refined the Event and Observer APIs to be clearer, easier to use, and more performant. We plan on rolling out Bevy's next generation Scene / UI system in the near future, and observers are a key piece! We wanted to ensure they were in a better place for the next phase of Bevy development. The old API had some problems:

1. **Concept names were confusing and ambiguous**: Events could be "observed", "buffered" in `Events` collections, or both. Knowing how to produce or consume a given [`Event`] required too much implied context: "do I write an Observer or use an EventReader system?", "do I trigger the event with or without targets?", what should the targets be?", etc. We need better, less ambiguous ways to refer to events.
2. **The API was not "static" enough**: This relates to (1). Because a given [`Event`] type could be used by and produced for _any context_, we had to provide access to _every possible API_ for _every event type_. It should not be possible to trigger an "entity event" without an entity! An Observer of an event that was not designed to have a target entity should not have an `entity()` field! Every [`Event`] impl had to define an "entity propagation traversal", even it was not designed to propagate (and even if it didn't target entities at all!). Events should be self documenting, impossible to produce or consume in the wrong context, and should only encode the information that is necessary for that event.
3. **The API did too much work**: Because events could be produced and used in any context, this meant that they all branched through code for every possible context. This incurred unnecessary overhead. It also resulted in lots of unnecessary codegen!

In **Bevy 0.17** we have sorted out these issues without fundamentally changing the shape of the API. Migrations should generally be very straightforward.

## The Rearchitecture

The `Event` trait has been reframed / refocused to increase flexibility, make the API more static, and remove specialized cruft:

```rust
// Old: Bevy 0.16
trait Event {
    // this embedded configuration specific to "propagating entity events" in all events!
    type Traversal: Traversal<Self>;
    const AUTO_PROPAGATE: bool = false;
    fn register_component_id(world: &mut World);
    fn component_id(world: &World) -> Option<ComponentId>;
}

// New: Bevy 0.17
trait Event {
    type Trigger<'a>: Trigger<Self>;
}
```

Every [`Event`] now has an associated [`Trigger`] implementation. The [`Trigger`] trait defines the behavior of `world.trigger()` for that event. [`Trigger`] defines which observers will run, the order they will run in, and the data that is passed to them.

By representing this in the type system, we can constrain behaviors and data to _specific_ types of events statically, making it impossible to "misuse" an [`Event`].
All of Bevy's existing "flavors" of events have been ported to the new [`Event`] / [`Trigger`] system.

## `Event`: global by default

At a glance, the default [`Event`] derive and usage hasn't changed much. Just some shorter / clearer naming. The old API looked like this:

```rust
#[derive(Event)]
struct GameOver {
    score: u32,
}

world.add_observer(|trigger: Trigger<GameOver>| {
    info!("Game over! You scored {} points", trigger.score);
});

world.trigger(GameOver { score: 100 });
```

In **Bevy 0.17**, defining observers has only changed slightly:

```rust

world.add_observer(|game_over: On<GameOver>| {
    info!("Game over! You scored {} points", game_over.score);
});

```

`Trigger` is now `On`. `On` encourages developers to think of this parameter _as the event itself_. This is also reflected in the new naming convention, where we name the variable after the `Event` (ex: `game_over`) rather than the `Trigger` (ex: `trigger`).

Internally things are a bit different though! The [`Event`] derive defaults to being "untargeted" / "global", by setting the `Event::Trigger` to [`GlobalTrigger`]. When it is triggered, only "untargeted" top-level observers will run, and there is _no way_ to trigger it in a different context (ex: events with a [`GlobalTrigger`] cannot target entities!).

## `EntityEvent`: a dedicated trait for entity-targeting events

In previous versions of Bevy, _any_ event could optionally be triggered for an entity. It looked like this:

```rust
#[derive(Event)]
struct Click;

world.trigger_targets(Click, entity);
```

In **Bevy 0.17**, if you want an [`Event`] to target an [`Entity`] (and thus trigger any observers watching for that specific entity), you derive [`EntityEvent`]:

```rust
#[derive(EntityEvent)]
struct Click {
    entity: Entity,
}

world.trigger(Click { entity });
```

Notice that `Click` now has the target entity as a field _on_ the [`Event`], and it now uses the same `world.trigger()` API that other events use. `world.trigger_targets` is no more ... every event is triggered using the same API!

```rust
// This observer will run for _all_ Click events targeting any entity
world.add_observer(|mut click: On<Click>| {});

/// This observer will only run for Click events triggered for `some_entity`
world.entity_mut(some_entity).observe(|mut click: On<Click>| {});
```

[`EntityEvent`] is a new trait:

```rust
trait EntityEvent: Event {
    fn event_target(&self) -> Entity;
    fn event_target_mut(&mut self) -> &mut Entity;
}
```

When it is derived, it defaults to setting the [`Event`] trigger to [`EntityTrigger`]. This will trigger all "untargeted" observers (`world.add_observer()`), just like [`GlobalTrigger`], but it will _also_ trigger any observers that target a specific entity (`world.entity_mut(some_entity).observe()`).

Deriving [`EntityEvent`] will set the `entity_target` to a field named `entity` by default. In some cases (such as events that have multiple entity fields), it might make sense to use a more descriptive name. You can set the target using the `#[event_target]` field attribute:

```rust
#[derive(EntityEvent)]
struct Attack {
    // This will trigger `attacker` observers 
    #[event_target]
    attacker: Entity,
    attacked: Entity,
}
```

## EntityEvent Propagation

An [`EntityEvent`] does not "propagate" by default (and they now statically have no access to APIs that control propagation). Propagation can be enabled using the `propagate` attribute (which defaults to using the [`ChildOf`] relationship to "bubble events up the hierarchy"):

```rust
#[derive(EntityEvent)]
#[entity_event(propagate)]
struct Click {
    entity: Entity
}
```

This will set the [`Event`]'s [`Trigger`] to [`PropagateEntityTrigger`].

This enables access to "propagation" functionality like this:

```rust
world.add_observer(|mut click: On<Click>| {
    if SOME_CONDITION {
        // stop the event from "bubbling up"
        click.propagate(false);
    }
});
```

Bevy's `Pointer` events have always tracked the "original target" that an "entity event" was targeting. This was handy! We've enabled this functionality for every [`EntityEvent`] with [`PropagateEntityTrigger`]: simply call `On::original_event_target`.

## Component Lifecycle Events

In past releases, the observer API for lifecycle events looked like this:

```rust
app.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Added player {}", trigger.entity());
});
```

We've ported these over to the new system, and renamed them to match our new naming scheme (ex: `OnAdd` is now [`Add`]). They look like this now:

```rust
app.add_observer(|add: On<Add, Player>| {
    info!("Added player {}", add.entity);
});
```

Component lifecycle events are an [`EntityEvent`] (and thus store the target entity as a field). They use the [`EntityComponentsTrigger`], which allows them to be triggered for specific components on an entity.

## AnimationEvent

"Animation events" are custom events that are registered with an [`AnimationPlayer`] and triggered at a specific point in the animation. [`AnimationEvent`] is a new event sub-trait / derive (much like [`EntityEvent`]). Animation events use the [`AnimationEventTrigger`]. They behave like an [`EntityEvent`] (they observers on the [`AnimationPlayer`]), but they notably _do not store the entity on the event type_. This allows for directly registering them in [`AnimationPlayer`] without needing to set an entity target:

```rust
animation.add_event(
    0.0,
    PrintMessage("Hello".to_string()),
);

world.entity_mut(animation_player).observe(|print_message: On<PrintMessage>| {
    // The `AnimationEventTrigger` still provides access to the animation_player entity
    println!("{} says {}", print_message.trigger().animation_player, print_message.0);
});
```

## Custom Event Triggers

The new [`Trigger`] trait also enables developers to implement their _own_ specialized [`Event`] [`Trigger`] logic.

The [`Event`] derive can specify a custom [`Trigger`] like this:

```rust
#[derive(Event)]
#[event(trigger = CoolTrigger)
struct Jump;
```

Alternatively, developers can create specialized event derives / traits, following the same pattern as `EntityEvent`:

```rust
trait CoolEvent: Event { }

#[derive(CoolEvent)]
struct Jump;

// the derive above would generate this code:
impl CoolEvent for Jump {}
impl Event for Jump {
    type Trigger<'s> = CoolTrigger; 
}
```

## Concept Clarity: Events vs Messages

In previous versions of Bevy, the [`Event`] trait was used for both "observable events" (handled with `Observer`) and "buffered events" (handled with `EventReader`). This made _some_ sense, as both concepts could be considered "events" in their own right. But they are also fundamentally _very_ different things functionally:

1. "Observable events" are consumed one-by-one in Observers, which exist outside of a schedule. "Buffered events" are consumed by iterating over many of them in normal systems, which exist in one or more places inside a schedule.
2. "Observable event" handlers are run _for_ developers. "Buffered event" consumers are responsible for dispatching handler logic themselves.
3. "Observable events" are handled immediately. "Buffered events" are handled at some later moment in time (or not at all).
4. "Observable events" need additional configuration to make them work (ex: `Event::Trigger`). "Buffered events" do not.
5. "Observable events" incur a small amount of per-handler overhead. Handling "buffered events" is as fast as iterating an array.

Most importantly: there was _no way_ for consumers or producers of these events to know _how_ to handle them, just by looking at the type info. Consider some `ProcessingFinished` event from some 3rd party library. Events could either be "buffered" or "observed" (depending on what the sender of the event chooses), so the consumer has _no way_ to know how to consume `ProcessingFinished`. Is their observer not firing because the event isn't happening, or because the creator of the event was sending it as a buffered event instead of "triggering" it?

These are two completely separate systems, with different producer / consumer APIs, different performance considerations, and immediate vs deferred handling. The "things" being sent deserve different concept names to solidify conceptually (and at the type/API level) their intended purpose and context.

In **Bevy 0.17**, [`Event`] is now _exclusively_ the name/trait for the concept of something that is "triggered" and "observed". [`Message`] is the name / trait of something that "buffered": it is "written" via a [`MessageWriter`] and "read" via a [`MessageReader`].

It is still possible to support both contexts by implementing _both traits_, but we expect that to be significantly less common than just choosing one.

[`Event`]: https://dev-docs.bevy.org/bevy/ecs/event/trait.Event.html
[`Trigger`]: https://dev-docs.bevy.org/bevy/ecs/event/trait.Trigger.html
[`GlobalTrigger`]: https://dev-docs.bevy.org/bevy/ecs/event/struct.GlobalTrigger.html
[`EntityEvent`]: https://dev-docs.bevy.org/bevy/ecs/event/trait.EntityEvent.html
[`ChildOf`]: https://dev-docs.bevy.org/bevy/ecs/hierarchy/struct.ChildOf.html
[`PropagateEntityTrigger`]: https://dev-docs.bevy.org/bevy/ecs/event/struct.PropagateEntityTrigger.html
[`Add`]: https://dev-docs.bevy.org/bevy/ecs/lifecycle/struct.Add.html
[`EntityComponentsTrigger`]: https://dev-docs.bevy.org/bevy/ecs/event/struct.EntityComponentsTrigger.html
[`AnimationPlayer`]: https://dev-docs.bevy.org/bevy/animation/struct.AnimationPlayer.html
[`AnimationEvent`]: https://dev-docs.bevy.org/bevy/animation/trait.AnimationEvent.html
[`AnimationEventTrigger`]: https://dev-docs.bevy.org/bevy/animation/struct.AnimationEventTrigger.html
