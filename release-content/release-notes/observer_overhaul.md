---
title: Observer Overhaul
authors: ["@Jondolf", "@hukasu"]
pull_requests: [19596, 19611]
---

## Rename `Trigger` to `On`

In past releases, the observer API looked like this:

```rust
app.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Added player {}", trigger.target());
});
```

In this example, the `Trigger` type contains information about the `OnAdd` event that was triggered
for a `Player`.

**Bevy 0.17** renames the `Trigger` type to `On`, and removes the `On` prefix from lifecycle events
such as `OnAdd` and `OnRemove`:

```rust
app.add_observer(|trigger: On<Add, Player>| {
    info!("Added player {}", trigger.target());
});
```

This significantly improves readability and ergonomics, and is especially valuable in UI contexts
where observers are very high-traffic APIs.

One concern that may come to mind is that `Add` can sometimes conflict with the `core::ops::Add` trait.
However, in practice these scenarios should be rare, and when you do get conflicts, it should be straightforward
to disambiguate by using `ops::Add`, for example.

## Observers with derived names

While debugging an app with many observers using tools like [`bevy_inspector_egui`](https://crates.io/crates/bevy-inspector-egui)
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
