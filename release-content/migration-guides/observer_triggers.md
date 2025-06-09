---
title: Observer Triggers
pull_requests: [19440]
---

Observers may be triggered on particular entities or globally.
Previously, a global trigger would claim to trigger on a particular `Entity`, `Entity::PLACEHOLDER`.
For correctness and transparency, triggers have been changed to `Option<Entity>`.

`Trigger::target` now returns `Option<Entity>` and `ObserverTrigger::target` is now of type `Option<Entity>`.
If you were checking for `Entity::PLACEHOLDER`, migrate to handling the `None` case.
If you were not checking for `Entity::PLACEHOLDER`, migrate to unwrapping, as `Entity::PLACEHOLDER` would have caused a panic before, at a later point.
