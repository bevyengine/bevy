---
title: "`Pressed` Component is now a newtype bool component"
pull_requests: [25154]
---

Previously, the `Pressed` component was a marker component without any fields. Now, it is a marker component with a single boolean field signifying whether the entity is being pressed. This was done to enable usage of change detection of the `Pressed` component in regular systems. The `Pressed` component is still removed from entities, but only after one whole frame of staying in the `false` state.

To check whether an entity is being pressed, the `Pressed` component must be present on the entity **and** its boolean field must be true. A convenience trait for `Option<&Pressed>` and `Option<Mut<'_, Pressed>>` types is available at `ui::interaction_states::OptionPressedExt` to make this check easier to reference.

```rust
// Before
fn system(
  query: Query<Has<Pressed>>
) {
  for is_pressed in query {
    if is_pressed {
      // ...
    }
  }
}

// After
use bevy::ui::interaction_states::OptionPressedExt;

fn system(
  query: Query<Option<&Pressed>>
) {
  for maybe_pressed in query {
    if maybe_pressed.is_pressed() {
      // ...
    }
  }
}
```

If you were using `bevy::ecs::lifecycle::RemovedComponents` in systems to check for the removal of `Pressed`, you can now filter queries with `Changed<Pressed>` and check for `!maybe_pressed.is_pressed()` to change your app's behavior based on press release.
