---
title: "Named UiSubZLayers"
pull_requests: [26260]
---

Instead of using explicit `f32` z-offsets, `bevy_ui_render` now has named sub z layers.
The z offset constants in the `stack_z_offsets` module have been replaced by the `UiSubZLayers` enum.
The list of layers is stored in back-to-front order in the `UiSubZLayerList` resource.
During extraction the zoffsets for each renderable element is looked up from `UiSubZLayerList`.

The `UiMaterial` trait's `stack_z_offset` method has been replaced by the `sub_z_layer`, which should return the `UiSubZlayer` label the material should be drawn to.
