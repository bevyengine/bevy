---
title: Parent Picking
pull_requests: [18982]
---

`target` on `Trigger<Pointer<E>>` has been renamed to `original_target`.

Previously `.target()` and `.target` would refer to two separate entities, the root and the leaf of the entity hierarchy.

See `examples/picking/parent_picking.rs` for a demonstration of this.
