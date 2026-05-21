---
title: Interactive Transform Gizmo
authors: ["@jbuehler23", "@aevyrie"]
pull_requests: [23435]
---

*TODO: Add a screenshot or GIF of the transform gizmo in use in the viewport.*

A transform gizmo — the click-and-drag handles for translating, rotating, and scaling objects in a 3D viewport — is one of the first things anyone reaches for when building a level editor. Bevy now has one built in, for your use today and our own use in the future.

Add `TransformGizmoPlugin`, mark a camera with `TransformGizmoCamera`, and tag entities with `TransformGizmoFocus`:

```rust
app.add_plugins(TransformGizmoPlugin);

commands.spawn((Camera3d::default(), TransformGizmoCamera));
commands.spawn((Mesh3d(mesh), TransformGizmoFocus));
```

The plugin is deliberately not connected to user input.
This keeps the gizmo composable for editor authors who already have opinions about input handling. Sensitivity, snapping, and screen-space scaling are all configurable via `TransformGizmoConfig`,
while modes are controlled via the `TransformGizmoMode` resource.

Much of the math and implementation strategy for this widget comes from the [`bevy_transform_gizmo`](https://github.com/fslabs/bevy_transform_gizmo) crate.
Thanks again to Foresight Spatial Labs for their generous open source contributions!
