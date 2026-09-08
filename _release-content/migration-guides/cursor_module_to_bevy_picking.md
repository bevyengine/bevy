---
title: "`cursor` module moved from `bevy_feathers` to `bevy_picking`"
pull_requests: [25294]
---

The `bevy_feathers` `cursor` module, containing `EntityCursor`, `DefaultCursor`, `OverrideCursor`, and `CursorIconPlugin` have been moved
from `bevy_feathers::cursor` to `bevy_picking::cursor`.

The `custom_cursor` feature has also been moved to `bevy_picking`.

Before:

```rust
use bevy_feathers::cursor::{CursorIconPlugin, DefaultCursor, EntityCursor, OverrideCursor};
```

After:

```rust
use bevy_picking::cursor::{CursorIconPlugin, DefaultCursor, EntityCursor, OverrideCursor};
```
