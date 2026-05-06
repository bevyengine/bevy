---
title: Delayed Commands
authors: ["@Runi-c"]
pull_requests: [23090]
---

Scheduling things to happen some time in the future is a common and useful tool in game development
for everything from gameplay logic to audio cues to VFX.

While this was previously possible through careful use of timers,
getting the details right was surprisingly tricky and naive solutions were heavy on boilerplate.

To make this easier, we now ship a general mechanism for delaying commands to be executed after a specified duration.

```rust
fn delayed_spawn(mut commands: Commands) {
    commands.delayed().secs(1.0).spawn(DummyComponent);
}

fn delayed_spawn_then_insert(mut commands: Commands) {
    let mut delayed = commands.delayed();
    let entity = delayed.secs(0.5).spawn_empty().id();
    delayed.secs(1.5).entity(entity).insert(DummyComponent);
}
```

Our goal is to provide a "good-enough" system for simple use-cases.
As a result, there are certain non-configurable implementation details — for example, delayed commands are currently always ticked by the default
clock during `PreUpdate` (typically `Time<Virtual>`).

If you need something more bespoke, take a look at our implementation,
then use the new helpers added for this feature and roll your own customized version of delayed commands.
