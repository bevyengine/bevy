---
title: Interactive Transform Gizmo
authors: ["@jbuehler23"]
pull_requests: [23435]
---

*TODO: Add a screenshot or GIF of the transform gizmo in use in the viewport.*

An opt-in interactive transform gizmo is now available for translating, rotating, and scaling entities in the viewport. This is useful for editor-like workflows, level design tools, and rapid prototyping.

Add `TransformGizmoPlugin`, mark a camera with `TransformGizmoCamera`, and tag entities with `TransformGizmoFocus`:

```rust
app.add_plugins(TransformGizmoPlugin);

commands.spawn((Camera3d::default(), TransformGizmoCamera));
commands.spawn((Mesh3d(mesh), TransformGizmoFocus));
```

The plugin is deliberately not connected to user input.
This keeps the gizmo composable for editor authors who already have opinions about input handling. Sensitivity, snapping, and screen-space scaling are all configurable via `TransformGizmoConfig`,
while modes are controlled via the `TransformGizmoMode` resource.
