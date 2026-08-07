---
title: UI picking now ignores transparent pixels of image nodes
pull_requests: [25077]
---

UI picking now ignores transparent regions of `ImageNode`s by default, matching the behavior of
sprite picking. A pointer over a pixel of an image node with an alpha value less than or equal to
`0.1` will no longer register that node as a hit.

This is controlled by the new `UiPickingSettings::picking_mode` field, which defaults to
`UiPickingMode::AlphaThreshold(0.1)`.

To restore the previous behavior (picking based on the node's bounding box, regardless of
transparency), set the picking mode to `UiPickingMode::BoundingBox`:

```rust
app.insert_resource(UiPickingSettings {
    picking_mode: UiPickingMode::BoundingBox,
    ..Default::default()
});
```

You can also override the mode for individual nodes by adding the `UiPickingMode` component:

```rust
commands.spawn((
    ImageNode::new(image),
    // This node will use a bounding-box check even though the global default is
    // an alpha threshold.
    UiPickingMode::BoundingBox,
));
```

Note that `UiPickingSettings` has a new `picking_mode` field. If you were constructing it with an
explicit struct literal, you must now provide this field (or use `..Default::default()`).
