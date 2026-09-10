---
title: MainEntityHashMap/MainEntityHashSet alias update
pull_requests: [18408]
---

`MainEntityHashSet`/`MainEntityHashMap` are now aliases of `EntityEquivalentHashSet`/`EntityEquivalentHashMap` and implement `EntitySet`.

As they are no longer aliases of the `hashbrown` types, some associated functions have to use the proper names or aliases in place of `HashMap`.
Example: `HashMap::default` -> `MainEntityHashMap::default`

Types associated with any of `EntityHashSet`, `EntityHashMap`, `EntityIndexSet`, `EntityIndexMap` now have an additional `K` generic. To maintain the previous meaning, use `Entity` for `K`.
