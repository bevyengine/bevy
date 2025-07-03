---
title: Hook into every `ApplyDeferred` using `OnDeferred`
authors: ["@andriyDev"]
pull_requests: []
---

Bevy now allows you to execute some code whenever `ApplyDeferred` is executed. This can be thought
of as a command that executes at every sync point.

To use this, first init the `OnDeferred` resource (to ensure it exists), then add to it:

```rust
app.init_resource::<OnDeferred>();
app.world_mut().resource_mut::<OnDeferred>().add(|world: &mut World| {
    // Do command stuff.
});
```

For one potential example, you could send commands through a channel to your `OnDeferred` command.
Since it has access to `&mut World` it can then apply any commands in the channel. While this is now
supported, more standard approaches are preferable (e.g., creating a system that polls the channel).
This is provided for situations where users truly need to react at every sync point.
