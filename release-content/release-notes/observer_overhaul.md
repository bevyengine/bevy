---
title: Observer Overhaul
authors: ["@Jondolf", "@alice-i-cecile", "@hukasu", "oscar-benderstone", "Zeophlite", "gwafotapa"]
pull_requests: [19596, 19663, 19611, 19935, 20274]
---

TODO: merge with Event split release notes

## Rename `Trigger` to `On`

In past releases, the observer API looked like this:

```rust
app.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Added player {}", trigger.entity());
});
```

In this example, the `Trigger` type contains information about the `OnAdd` event that was triggered
for a `Player`.

**Bevy 0.17** renames the `Trigger` type to `On`, and removes the `On` prefix from lifecycle events
such as `OnAdd` and `OnRemove`:

```rust
app.add_observer(|event: On<Add, Player>| {
    info!("Added player {}", event.entity());
});
```

This significantly improves readability and ergonomics, and is especially valuable in UI contexts
where observers are very high-traffic APIs.

One concern that may come to mind is that `Add` can sometimes conflict with the `core::ops::Add` trait.
However, in practice these scenarios should be rare, and when you do get conflicts, it should be straightforward
to disambiguate by using `ops::Add`, for example.

## Original targets

`bevy_picking`'s `Pointer` events have always tracked the original target that an entity-event was targeting,
allowing you to bubble events up your hierarchy to see if any of the parents care,
then act on the entity that was actually picked in the first place.

This was handy! We've enabled this functionality for all entity-events: simply call `On::original_entity`.

## Expose name of the Observer's system

The name of the Observer's system is now accessible through `Observer::system_name`,
this opens up the possibility for the debug tools to show more meaningful names for observers.

## Use `EventKey` instead of `ComponentId`

Internally, each `Event` type would generate a `Component` type, allowing us to use the corresponding `ComponentId` to track the event.
We have newtyped this to `EventKey` to help separate these concerns.

## Watch multiple entities

To watch multiple entities with the same observer you previously had to call `Observer::with_entity` or `Observer::watch_entity` for each entity. New methods `Observer::with_entities` and `Observer::watch_entities` have been added for your convenience.
