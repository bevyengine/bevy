---
title: Delayed Commands
authors: ["@Runi-c"]
pull_requests: [23090]
---

Scheduling things to happen some time in the future is a common and useful tool in game development
for everything from gameplay logic to audio cues to VFX.

While this was previously possible through careful use of timers,
getting the details right was surprisingly tricky and naive solutions were heavy on boilerplate.

Now, you can simply delay arbitrary commands to be executed later.

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

Note that this does not have a built-in, blessed cancellation mechanism yet.
We recommend embedding the originating `Entity` into the command if you want to cancel the action if that entity dies or is despawned.
