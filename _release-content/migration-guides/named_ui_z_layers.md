---
title: "`stack_z_offsets` has been replaced by named z layers"
pull_requests: [26260]
---

Instead of using explicit `f32` z-offsets, `bevy_ui_render` now has named paint layers.
The z offset constants in the `stack_z_offsets` module have been replaced by named `PaintLayer` constants.
The list of layers is stored in back-to-front order in the `PaintOrder` resource.
During extraction the zoffsets for each renderable element is looked up from `PaintOrder`.

The `UiMaterial` trait's `stack_z_offset` method has been replaced by `paint_layer`, which should return the `PaintLayer` the material is intended to be drawn to.
