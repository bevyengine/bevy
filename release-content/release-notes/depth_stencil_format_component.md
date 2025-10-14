---
title: DepthStencilFormat added
authors: ["@raketenben"]
pull_requests: [21493]
---

## DepthStencilFormat

Until now, Bevy has been using a fixed TextureFormat for the depth buffer, making the usage of stenciling with the main pipeline difficult.

This version introduces the `DepthStencilFormat` component, which can be added to a camera to change the depth/stencil format used during rendering.

This can be used to:
- lower precision of the depth buffer for increased performance (on some hardware)
- switching the pipeline to hardware-support-dependent formats like `Depth32FloatStencil8`
- enable the use of stenciling by choosing a format that includes a stencil aspect

By default, the format remains the same as in previous versions of bevy (`Depth32Float`).

```rust
commands.spawn((
	Camera3d::default(),
	DepthStencilFormat::Depth24PlusStencil8
));
```