---
title: Specialized Ui Transform
pull_requests: [16615]
---
New specialized 2D UI transform components `UiTransform` and `UiGlobalTransform`.  `UiTransform` is a 2d-only equivalent of `Transform` with a translation in `Val`s. `UiGlobalTransform` newtypes `Affine2` and is updated in `ui_layout_system`.
`Node` now requires `UiTransform` instead of `Transform`. `UiTransform` requires `UiGlobalTransform`.

In previous versions of Bevy `ui_layout_system` would overwrite UI node's `Transform::translation` each frame. `UiTransform`s aren't overwritten and there is no longer any need for systems that cache and rewrite the transform for translated UI elements. 

`RelativeCursorPosition`'s coordinates are now object-centered with (0,0) at the the center of the node and the corners at  (±0.5, ±0.5). Its `normalized_visible_node_rect` field has been removed and replaced with a new `cursor_over: bool` field which is set to true when the cursor is hovering an unclipped area of the UI node.