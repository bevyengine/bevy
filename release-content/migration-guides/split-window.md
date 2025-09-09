---
title: Window is now split into multiple components
pull_requests: [19668]
---

`Window` has become a very large component over the last few releases. To improve our internal handling of it and to make it more approachable, we
have split it into multiple components, all on the same entity. So far, this affects `CursorOptions`:

```rust
// old
fn lock_cursor(primary_window: Single<&mut Window, With<PrimaryWindow>>) {
    primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
}

// new
fn lock_cursor(primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    primary_cursor_options.grab_mode = CursorGrabMode::Locked;
}
```

This split also applies when specifying the initial settings for the primary window:

```rust
// old
app.add_plugins(DefaultPlugins.set(WindowPlugin {
    primary_window: Some(Window {
        cursor_options: CursorOptions {
            grab_mode: CursorGrabMode::Locked,
            ..default()
        },
        ..default()
    }),
    ..default()
}));

// new
app.add_plugins(DefaultPlugins.set(WindowPlugin {
    primary_cursor_options: Some(CursorOptions {
        grab_mode: CursorGrabMode::Locked,
        ..default()
    }),
    ..default()
}));
```
