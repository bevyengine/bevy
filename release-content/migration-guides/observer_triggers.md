---
title: Observer Triggers
pull_requests: [19440]
---

The `Trigger` type used inside observers has been renamed to `On` for a cleaner API.

```rust
// Old
commands.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Spawned player {}", trigger.target());
});

// New
commands.add_observer(|trigger: On<Add, Player>| {
    info!("Spawned player {}", trigger.target());
});
```

Observers may be triggered on particular entities or globally.
Previously, a global trigger would claim to trigger on a particular `Entity`, `Entity::PLACEHOLDER`.
For correctness and transparency, triggers have been changed to `Option<Entity>`.

`On::target` (previously `Trigger::target`) now returns `Option<Entity>`, and `ObserverTrigger::target`
is now of type `Option<Entity>`. If you were checking for `Entity::PLACEHOLDER`, migrate to handling the `None` case.
If you were not checking for `Entity::PLACEHOLDER`, migrate to unwrapping, as `Entity::PLACEHOLDER`
would have caused a panic before, at a later point.
