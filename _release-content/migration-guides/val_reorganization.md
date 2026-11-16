---
title: "Val reorganization"
pull_requests: [26415]
---

`LayoutContext` has been renamed to `ValContext`.

The `resolve` methods on `Val`, `Val2`, `UiPosition`, `CornerRadius`, `RadialGradientShape` and `BorderRadius`, and `UiTransform::compute_affine`, now take `&ValContext` instead of separate scale factor, target size, `EmSize`, and `RemSize` arguments. The size used as the basis for percent values remains a separate argument.

```rust
// 0.19
let position = ui_position.resolve(scale_factor, node_size, viewport_size, em_size, rem_size);

// 0.20
let context = ValContext::new(scale_factor, viewport_size, em_size, rem_size);
let position = ui_position.resolve(&context, node_size);
```

`Val`, `ValContext`, `ValParseError`, `ValArithmeticError`, `ValNum` and the `px`, `percent`, etc. constructors now live in a new `val` module, but are still re-exported from `bevy::ui` and the prelude, so imports don't need to change.
