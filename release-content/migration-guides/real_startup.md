---
title: `Startup` is no longer automatically called during `update` or custom runners.
pull_requests: [20407]
---

Previously, when calling `App::update`, schedules like `PreStartup`, `Startup`, and `PostStartup`
would run automatically. In Bevy 0.17, this is no longer the case. You can use `App::startup` to
explicitly trigger these schedules (though you should avoid calling them repeatedly).

In regards to `App::update`, this primarily impacts tests. If your test is doing something like:

```rust
app.world_mut().spawn(Transform::from_xyz(1.0, 2.0, 3.0));

app.update();

// Change something.

app.update();

// Change something else.

app.update();
```

You may now need to do:

```rust
app.world_mut().spawn(Transform::from_xyz(1.0, 2.0, 3.0));

app.startup();

app.update();

// Change something.

app.update();

// Change something else.

app.update();
```

For a vast majority of tests, this will be unnecessary. However a handful of features will require
`App::startup` to behave correctly (e.g., `bevy_input_focus`, `GlobalTransform`s for the first
frame).

Additionally, if you are using a custom runner for your `App`, you likely need to explicitly call
`App::startup`. Generally, custom runners should call `App::finish`, then `App::cleanup`, and (now)
`App::startup`.

```rust
fn my_custom_runner(mut app: App) -> AppExit {
    app.finish();
    app.cleanup();
    // **NEW**: Add this to your custom runners!
    app.startup();

    loop {
        app.update();
        if let Some(exit) = app.should_exit() {
            return exit;
        }
    }
}
```
