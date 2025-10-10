---
title: Resources as Components
pull_requests: [21346]
---

In 0.18, resources are no longer stored separately from the rest of the entities in the world.
Instead, resources are wrapped in the `ResourceComponent<_>` component and stored on singleton entities.
This means that each resource is stored on a separate entity.

This is largely an internal change which has little effect on user code, see the migration guide for all of the changes.
There are, however, some benefits for end users.
The largest advantage is that all of the ECS machinery for components, becomes available for resources.
The primary benefit is that observers now work for resources.
We can, for example, react to lifecycle events on a resource:

```rust
app.add_observer(|trigger: Trigger<OnAdd, ResourceComponent<SomeResource>>| {
    info!("Added resource {}", trigger.entity());
});
```
