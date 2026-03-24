---
title: Interactive Transform Gizmo
authors: ["@jbuehler23"]
pull_requests: [23435]
---

An opt-in interactive transform gizmo is now available for translating, rotating, and scaling entities in the viewport. This is useful for editor-like workflows, level design tools, and rapid prototyping.

Add `TransformGizmoPlugin`, mark a camera with `TransformGizmoCamera`, and tag entities with `TransformGizmoFocus`:

```rust
app.add_plugins(TransformGizmoPlugin);

commands.spawn((Camera3d::default(), TransformGizmoCamera));
commands.spawn((Mesh3d(mesh), TransformGizmoFocus));
```

Switch between modes by setting the `TransformGizmoMode` resource. The plugin does not handle keyboard input, you wire up controls however you like. Sensitivity, snapping, and screen-space scaling are configurable via `TransformGizmoConfig`.
