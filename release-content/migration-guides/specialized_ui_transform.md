---
title: Specialized Ui Transform
pull_requests: [16615]
---

In Bevy UI Transform and GlobalTransform have been replaced by UiTransform and UiGlobalTransform. UiTransform is a specialized 2D UI transform which supports responsive translations. Val::Px values are equivalent to the previous unitless translation components.

In previous versions the Transforms of UI nodes would be overwritten by ui_layout_system each frame. UiTransforms aren't modified, so there is no longer any need for systems that cache and rewrite the transform for translated UI elements.

RelativeCursorPosition's coordinates are now object-centered with (0,0) at the the center of the node and the corners at (±0.5, ±0.5). Its normalized_visible_node_rect field has been removed and replaced with a boolean value cursor_over which is set to true when the cursor is hovering an unclipped area of the UI node.