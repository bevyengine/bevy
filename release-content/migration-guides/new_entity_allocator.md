---
title: Entities Utilities
pull_requests: [18670]
---

`Entities::reserve` has been renamed `Entities::prepare`, as it has looser guarantees.

Additionally, `Entities` debug methods `used_count` and `total_prospective_count` have been removed.
This is because the new allocator is much more flexible, which makes it unrealistic to track these quantities (and less meaningful).

`Entities` debug methods `total_count` and `len` now return `u32` instead of `usize`.
Since `Entities` has a well defined upper bound, unlike other collections, it makes more since to use `u32` explicitly rather than `usize`.

To migrate:

```diff
- let entities: usize = entities.len();
+ let entities: u32 = entities.len();
```

```diff
- entities.reserve(128);
+ entities.prepare(128);
```

If you have any trouble migrating away from `Entities::used_count` and `Entities::total_prospective_count`, feel free to open an issue!
