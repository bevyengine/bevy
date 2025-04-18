---
title: Entities Utilities
pull_requests: [18670]
---

`Entities::reserve` has been renamed `Entities::prepare`. Additionally, `Entities` methods `used_count` and `total_prospective_count` have been removed, and `total_count` and `len` now return `u64` instead of `usize`.

These utility methods have changed because the backing entity allocator has had a rewrite. `Entities::prepare` is intentionally more generally named than `Entities::reserve` because it has looser guarantees, and it may do more than just reserving memory in the future. `Entities::used_count` and `Entities::total_prospective_count` were removed because they depend on knowing how many entities are pending being automatically flushed. However, tracking that quantity is now nontrivial, and these functions have always been intended for debugging use only. The new allocator allows entities to be reserved without them being added to the pending list for automatic flushing, and it allows pending entities to be manually flushed early. Effectively, that means debugging the entities that are pending is no longer relevant information, hence the removal of those methods. `total_count` and `len` now return `u64` instead of `usize` to better reflect the truth. Since `Entities` has a well defined upper bound, unlike other collections, it makes more since to use `u64` explicitly rather than `usize`.

To migrate:

```diff
- let entities: usize = entities.len();
+ let entities: u64 = entities.len();
```

```diff
- entities.reserve(128);
+ entities.prepare(128);
```

If you have any trouble migrating away from `Entities::used_count` and `Entities::total_prospective_count`, feel free to open an issue!
