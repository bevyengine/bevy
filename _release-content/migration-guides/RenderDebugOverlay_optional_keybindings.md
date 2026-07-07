---
title: RenderDebugOverlay added to default plugins and now has optional keybindings
pull_requests: [24891]
---

RenderDebugOverlay is now added to the default plugins, disabled by default. enable it like this:

```rust
App::new()
        .add_plugins(DefaultPlugins.set(RenderDebugOverlayPlugin {
            enable_keybindings: true,
        }))
```

keybindings are set to `KeyCode::F1` for cycling modes and `KeyCode::F2` for cycling opacity by default.

keybindings are configurable during runtime by changing the `RenderDebugOverlay` resource:

```rust
    App::new()
        .add_plugins(DefaultPlugins.set(RenderDebugOverlayPlugin {
            enable_keybindings: true,
        }))
        .insert_resource(RenderDebugOverlayKeybindings {
            cycle_mode: KeyCode::F3,
            cycle_opacity: KeyCode::F4,
        })
```

Keybindings can be changed at runtime, for example:

```rust
fn change_keybindings(mut keybindings: ResMut<RenderDebugOverlayKeybindings>) {
    keybindings.cycle_mode = KeyCode::F5;
    keybindings.cycle_opacity = KeyCode::F6;
}
```
