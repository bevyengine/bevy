---
title: `ViewportNode`
authors: ["@chompaa", "@ickshonpe"]
pull_requests: [17253]
---

Bevy UI now has a `ViewportNode` component, which lets you render camera output directly to a UI node. Furthermore, if the `bevy_ui_picking_backend` feature is enabled, you can pick using the rendered target. That is, you can use **any** picking backend through the viewport node, as per normal. In terms of UI, the API usage is really straightforward:

```rust
commands.spawn((
  // `ViewportNode` requires `Node`, so we just need this component!
  ViewportNode::new(camera)
  // To disable picking "through" the viewport, just disable picking for the node.
  // Pickable::IGNORE
));
```

The referenced `camera` here does require its target to be a `RenderTarget::Image`. See the new [`viewport_node`](https://github.com/bevyengine/bevy/blob/v0.17.0/examples/ui/viewport_node.rs) for more implementation details.

## Showcase

`https://private-user-images.githubusercontent.com/26204416/402285264-39f44eac-2c2a-4fd9-a606-04171f806dc1.mp4?jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJnaXRodWIuY29tIiwiYXVkIjoicmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbSIsImtleSI6ImtleTUiLCJleHAiOjE3NDU4NTY4MDgsIm5iZiI6MTc0NTg1NjUwOCwicGF0aCI6Ii8yNjIwNDQxNi80MDIyODUyNjQtMzlmNDRlYWMtMmMyYS00ZmQ5LWE2MDYtMDQxNzFmODA2ZGMxLm1wND9YLUFtei1BbGdvcml0aG09QVdTNC1ITUFDLVNIQTI1NiZYLUFtei1DcmVkZW50aWFsPUFLSUFWQ09EWUxTQTUzUFFLNFpBJTJGMjAyNTA0MjglMkZ1cy1lYXN0LTElMkZzMyUyRmF3czRfcmVxdWVzdCZYLUFtei1EYXRlPTIwMjUwNDI4VDE2MDgyOFomWC1BbXotRXhwaXJlcz0zMDAmWC1BbXotU2lnbmF0dXJlPTg0ZDU0OGFmM2Q3NTJmOWJkNDYzODMxNjkyOTBlYzFmNmQ2YWUzMGMzMjJjMjFiZWI0ZmY3ZjZkMjNiMzA5NzkmWC1BbXotU2lnbmVkSGVhZGVycz1ob3N0In0.DXec6l2SYDIpSCRssEB4o3er7ib3jUQ9t9fvjdY3hYw`
