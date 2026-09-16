---
title: Explicit TypeId map aliases
pull_requests: [25053]
---

`TypeIdHashMap` and `TypeIdIndexMap` have been added for code that maps
[`TypeId`](https://doc.rust-lang.org/std/any/struct.TypeId.html) values. Use `TypeIdHashMap` when
iteration order is unimportant and average O(1) removal is desired. Use `TypeIdIndexMap` when
insertion-order iteration is required.

`TypeIdMap` remains an alias for the ordered `TypeIdIndexMap`, but is deprecated so users can
choose the appropriate behavior explicitly. `TypeIdMapEntry` is likewise deprecated in favor of
`TypeIdHashMapEntry` or `TypeIdIndexMapEntry`. Use `TypeIdHashMapExt` for the generic convenience
methods on `TypeIdHashMap`; the existing `TypeIdMapExt` continues to work with ordered maps.
