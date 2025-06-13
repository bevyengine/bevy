---
title: Observers with derived name
authors: ["@hukasu"]
pull_requests: [19611]
---

While debugging an app with many observers using tools like [`bevy_inspector_egui](https://crates.io/crates/bevy-inspector-egui)
you would get inundated with entities just named `Observer`. Now observers will have a `Name` component derived from
the name of the function used to create it.

For the `my_game` crate you will now have this:
```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // This observer will be `Name` of `my_game::my_trigger`
        .add_observer(my_trigger)
        // This observer will be `Name` of `my_game::main::{{closure}}`
        .add_observer(|_trigger: Trigger<SceneInstanceReady>| {})
        .run();
}

fn my_trigger(_trigger: Trigger<SceneInstanceReady>) {}
```
