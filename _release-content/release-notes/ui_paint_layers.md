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

- `insert_before_layer` / `insert_after_layer` place a layer directly below or above an existing layer.
- `insert_top` / `insert_bottom` place a layer above or below all the other layers.
- `remove_layer` removes a layer. A built-in element whose layer is removed isn't drawn.
The insert methods move the layer if it's already present.

The `UiMaterial` trait has a new `paint_layer()` method that chooses the layer the layer the material is drawn to.
It defaults to `MATERIAL_PAINT_LAYER`.

`PaintOrder` just newtypes an `IndexSet`, so you can also use the `IndexSet` methods to reorder it arbitrarily.

Extraction functions should look up the z-offsets for their elements layer with `PaintOrder::get_offset`, which returns a value in the range `0.0..1.0`. The z-offset should be looked up once at the start of the extraction function, and shouldn't be looked up per node if at all possible.

The `PaintOrder` resource is also extracted separately, so the offsets can be applied in some queue functions instead for some elements. This is because extraction isn't as centralized as it should be.

Changing `PaintOrder` at all is expensive as it triggers a reextraction of all elements.
