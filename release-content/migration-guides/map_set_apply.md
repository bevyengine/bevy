---
title: `DynamicMap` is now unordered, `Map::get_at` and `Map::get_at_mut` are now removed, and `apply` removes excess entries from reflected maps.
pull_requests: [19802]
---

`DynamicMap` is now unordered, and the `Map` trait no longer assumes implementors to be ordered. If you previously relied on them being ordered, you should now store a list of keys (`Vec<Box<dyn PartialReflect>>`) separately.

`Map::get_at` and `Map::get_at_mut` are now removed. You should no longer use `usize` to index into the map, and instead use `&dyn PartialReflect` with `Map::get` and `Map::get_mut`.

`PartialReflect::apply(self, other)` for maps now removes excess entries (entries present in `self` which are not present in `other`).
If you need those entries to be preserved, you will need to re-insert them manually.
