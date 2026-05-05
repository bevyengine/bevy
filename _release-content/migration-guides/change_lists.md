---
title: "Change lists"
pull_requests: [22966]
---

Previously, Bevy required rendering phases to iterate over all visible entities to determine which objects changed via ticks. This became a bottleneck, so Bevy now uses *change lists* instead. This change affects custom render phases; if you aren't creating your own render phases, you shouldn't have to update any code.

In the render world, the list of changed items can now be accessed in the new `DirtySpecializations` resource. In *specialize* systems, use code like the following:

```rust
// First, remove meshes that need to be respecialized, and those that were removed, from the bins.
for &main_entity in dirty_specializations
    .iter_to_dequeue(view.retained_view_entity, render_visible_mesh_entities)
{
    opaque_phase.remove(main_entity);
}

// Specialize new meshes.
for (render_entity, visible_entity) in dirty_specializations.iter_to_queue(
    view.retained_view_entity,
    render_visible_mesh_entities,
    &view_pending_mesh_queues.prev_frame,
) {
    ...
}

```

In *queue* systems, use code like this:

```rust
// First, remove meshes that need to be respecialized, and those that were removed, from the bins.
for &main_entity in dirty_specializations
    .iter_to_dequeue(view.retained_view_entity, render_visible_mesh_entities)
{
    my_phase.remove(Entity::PLACEHOLDER, main_entity);
}

// Now bin new items.
for (render_entity, visible_entity) in dirty_specializations.iter_to_queue(
    view.retained_view_entity,
    render_visible_mesh_entities,
    &view_pending_mesh_queues.prev_frame,
) {
    ...
}
```

If you need to handle the case in which a mesh might not be able to be specialized and/or queued right away because its dependencies (e.g. materials) haven't loaded yet, there's a new type `PendingQueues` that can help with this.

Additionally, sorted render phases now use an `IndexMap` instead of a `Vec`, so that entities can be added and removed incrementally instead of having to reconstruct the list every frame. This is incompatible with some exotic sorting algorithms that were commonly in use before (e.g. radix sort), so you may need to switch to the built-in `sort_unstable` method on `IndexMap`.

See `examples/shader_advanced/specialized_mesh_pipeline.rs` for a comprehensive example.
