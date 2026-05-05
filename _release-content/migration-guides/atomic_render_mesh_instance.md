---
title: RenderMeshInstance becomes atomic
pull_requests: [22988]
---

In order to enhance the performance and scalability of `RenderMeshInstance`, its fields have been made atomic. Code that accessed fields like:

```rust
let instance: &RenderMeshInstance = ...;
... instance.mesh_asset_id ...
```

Should now use the accessor methods like this:

```rust
let instance: &RenderMeshInstance = ...;
... instance.mesh_asset_id() ...
```

There are associated setter methods with a `set_` prefix as well. Note that, because `RenderMeshInstance` fields are atomic, you don't need an `&mut` reference to call them. However, it's now your responsibility to avoid data races.
