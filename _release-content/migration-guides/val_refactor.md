---
title: "Val refactor"
pull_requests: [26409]
---

- `LayoutContext` has been renamed to `ValContext`.
- `Val::resolve`, `Val2::resolve`, `UiPosition::resolve`, `CornerRadius::resolve`,
  `RadialGradientShape::resolve`, `BorderRadius::resolve`, and `UiTransform::compute_affine`
  now take `&ValContext` instead of separate scale factor, target size, `EmSize`, and `RemSize`
  arguments. The basis for percent values remains a separate argument.
- `Val` has been moved from the `bevy_ui` crate's `geometry` module to a new module `val`.
- `TaffyStyle` no longer implements `Clone`.
