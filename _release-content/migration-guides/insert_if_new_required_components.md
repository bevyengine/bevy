---
title: "`insert_if_new` no longer re-adds required components of already-present components"
pull_requests: [24593]
---

The `InsertMode::Keep` insert APIs (`EntityWorldMut::insert_if_new`, `EntityCommands::insert_if_new`, `World::insert_batch_if_new`, `World::try_insert_batch_if_new`) now only add a required component when the component requiring it is actually being inserted.

Previously, inserting a component that was already present could still (re-)insert its required components. So if you removed a required component and then re-inserted its requirer with `insert_if_new`, the required component was silently brought back, even though the requirer itself was unchanged. Now inserting an already-present component is a true no-op, including for its required components.

```rust
#[derive(Component)]
#[require(B)]
struct A;
#[derive(Component, Default)]
struct B;

let id = world.spawn(A).id(); // inserts A and its required B
world.entity_mut(id).remove::<B>();
world.entity_mut(id).insert_if_new(A);
// Before: B is present again.
// Now:    B stays absent, because A was already present so the insert did nothing.
```

If you relied on the old behavior to restore a required component, insert it explicitly (`insert(B)`), or use `insert` (`InsertMode::Replace`) instead of `insert_if_new`.
