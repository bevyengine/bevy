---
title: "`stack_z_offsets` has been replaced by named z layers"
pull_requests: [26260]
---

Instead of using explicit `f32` z-offsets, `bevy_ui_render` now has named paint layers.
The z offset constants in the `stack_z_offsets` module have been replaced by named `PaintLayer` constants.
The list of layers is stored in back-to-front order in the `PaintOrder` resource.

`TransparentUi::sort_key` now uses `UiSortKey` instead of `FloatOrd`. UI phase items are ordered first by stack index, then by paint layer index. Use `PaintOrder::get_layer_index` to look up a layer's index, then construct a `UiSortKey` with `UiSortKey::new(stack_index, paint_layer)`

`ExtractedUiNode::z_order` has been replaced by `stack_index: ComputedStackIndex` and `paint_layer: usize`.
The `stack_index` fields on `ExtractedBoxShadow`, `ExtractedGradient`, `ExtractedUiMaterialNode`, and `ExtractedUiTextureSlice` now take `ComputedStackIndex` instead of `u32`.

The `UiMaterial` trait's `stack_z_offset` method has been replaced by `paint_layer`, which should return the `PaintLayer` the material is intended to be drawn to.
