---
title: Observer Triggers
pull_requests: [19440, 19596]
---

The `Trigger` type used inside observers has been renamed to `On` for a cleaner API.

```rust
// Old
commands.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Spawned player {}", trigger.entity());
});

// New
commands.add_observer(|event: On<Add, Player>| {
    info!("Spawned player {}", event.entity());
});
```

To reduce repetition and improve readability, the `OnAdd`, `OnInsert`, `OnReplace`, `OnRemove`, and `OnDespawn`
observer events have also been renamed to `Add`, `Insert`, `Replace`, `Remove`, and `Despawn` respectively.
In rare cases where the `Add` event conflicts with the `std::ops::Add` trait, you may need to disambiguate,
for example by using `ops::Add` for the trait.

Observers may be triggered on particular entities or globally.
Previously, a global trigger would claim to trigger on a particular `Entity`, `Entity::PLACEHOLDER`.
For correctness and transparency, triggers have been changed to `Option<Entity>`.

`On::entity` (previously `Trigger::target`) now returns `Option<Entity>`, and `ObserverTrigger::target`
is now of type `Option<Entity>`. If you were checking for `Entity::PLACEHOLDER`, migrate to handling the `None` case.
If you were not checking for `Entity::PLACEHOLDER`, migrate to unwrapping, as `Entity::PLACEHOLDER`
would have caused a panic before, at a later point.
