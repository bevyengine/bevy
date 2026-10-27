---
title: Paint layers
authors: ["@Ickshonpe"]
pull_requests: [26260]
---

`bevy_ui_render` now has paint layers.

The `PaintOrder` resource holds the list of paint layers, in the order they’re drawn from back to front.

The built-in layers are identified by named `PaintLayer` constants such as `BOX_SHADOW_PAINT_LAYER` and `TEXT_PAINT_LAYER`.

`UiRenderPlugin` initializes `PaintOrder` with all the builtin layers in the correct order.

To create a new `PaintLayer`, give it a string name:

```rust
pub const MY_PAINT_LAYER: PaintLayer = PaintLayer::new_static("my_plugin.overlay");
```

The `PaintOrder` resource exposes `IndexSet`’s methods through dereferencing, plus these helpers:

- `insert_before_layer` / `insert_after_layer` place a layer directly below or above an existing layer.
- `insert_top` / `insert_bottom` place a layer above or below all the other layers.
- `remove_layer` removes a layer. A built-in element whose layer is removed isn't drawn.

The insert methods move a layer if it is already present.

The `UiMaterial` trait has a new `paint_layer()` method that chooses the layer the layer the material is drawn to.
It defaults to `MATERIAL_PAINT_LAYER`.

UI render items are sorted using `UiSortKey`, first by `ComputedStackIndex` and then by paint-layer index.

Extraction functions should look up their layer index with `PaintOrder::get_layer_index` once at the start of the function. Extracted items store `ComputedStackIndex` directly, and `ExtractedUiNode` also stores the paint-layer index. Queue functions construct the sort key with `UiSortKey::new(stack_index, paint_layer)`. The `PaintOrder` resource is also extracted separately, so some queue functions look up the layer index there instead.

Changing `PaintOrder` at all is expensive, as it triggers a reextraction of all elements.
