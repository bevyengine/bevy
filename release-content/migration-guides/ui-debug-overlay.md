---
title: Move UI Debug Options from `bevy_ui` to `bevy_ui_render`
pull_requests: [18703]
---

The `UiDebugOptions` resource used for controlling the UI Debug Overlay has been moved from the internal `bevy_ui` crate to the `bevy_ui_render` crate, and is now accessible from the prelude of `bevy_ui_render` and, as before, from the prelude of `bevy`:

```rust
// 0.16
use bevy::prelude::*;
// or
use bevy::ui::UiDebugOptions;

// 0.17
use bevy::prelude::*;
// or, if you are not using the full `bevy` crate:
// use bevy_ui_render::prelude::*;

let options = world.resource_mut::<UiDebugOptions>();
```
