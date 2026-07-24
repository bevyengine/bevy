---
title: "`bevy_ui_widgets::observe` has been deprecated in favor of `bsn!`"
pull_requests: [23730]
---

The `observe` function and `AddObserver` struct in `bevy_ui_widgets` have been deprecated.
These were a workaround for attaching observers as bundle effects.
Now that `bsn!` supports the `on` helper natively, use that instead.

Before:

```rust
use bevy_ui_widgets::observe;

commands.spawn((
    Button,
    observe(|_event: On<Pointer<Press>>| {
        println!("Clicked!");
    }),
));
```

After:

```rust
commands.spawn(bsn! {
    Button
    on(|_event: On<Pointer<Press>>| {
        println!("Clicked!");
    })
});
```

If you were using `AddObserver` directly for some reason, replace it with `on` inside a `bsn!` block in the same way.
