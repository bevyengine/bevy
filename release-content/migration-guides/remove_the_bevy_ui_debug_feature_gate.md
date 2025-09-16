---
title: The `bevy_ui_debug` feature gate has been removed
pull_requests: [ 21091 ]
---

The `bevy_ui_debug` feature gate has been removed.
To use the debug overlay, add the `UiDebugOptions` resource to your Bevy app with its `enabled` field set to true:

```rust
    app.insert_resource(UiDebugOptions {
        enabled: true, 
        ..default()
    });

```