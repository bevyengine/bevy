---
title: "ComponentInfo no longer stores the component ID"
pull_requests: [25774]
---

`ComponentInfo::id()` has been removed, and `ComponentInfo` no longer stores the component ID. Component IDs are already used as keys by the collections that store `ComponentInfo` instances; if you need both values, retain the `ComponentId` when
retrieving the `ComponentInfo` from the collection:

```rust
// 0.19
let info = components.get_info(component_id).unwrap();
let id = info.id();

// 0.20
let id = component_id;
let info = components.get_info(id).unwrap();
```
