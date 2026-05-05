---
title: Observer Run Conditions
authors: ["@jonas-meyer"]
pull_requests: [22602]
---

Observers can now use run conditions with `.run_if()`, the same pattern systems use for conditional execution.

```rust
#[derive(Resource)]
struct GamePaused(bool);

// Observer only runs when game is not paused
app.add_observer(
    on_damage.run_if(|paused: Res<GamePaused>| !paused.0)
);

// Multiple conditions can be chained (AND semantics)
app.add_observer(
    on_damage
        .run_if(|paused: Res<GamePaused>| !paused.0)
        .run_if(resource_exists::<Player>)
);
```

This works with `add_observer`, entity `.observe()`, and the `Observer` builder pattern.
