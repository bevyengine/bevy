---
title: "Custom 2D and UI shaders encode for the view's compositing space"
pull_requests: [25625]
---

Shaders that draw into a view's main texture now encode their output for the view's
`CompositingSpace`. Without the encode call, a custom shader renders wrong colors on
`Srgb` and `Oklab` views.

A custom `Material2d` shader should call `writer_encode` on its output color as the last
step of its fragment shader. This replaces the `SRGB_OUTPUT` and `OKLAB_OUTPUT` shader
defs, which are now `COMPOSITING_SPACE_SRGB` and `COMPOSITING_SPACE_OKLAB`.

```wesl
import bevy_render::writer_encode::writer_encode;

return writer_encode(color);
```

A custom `UiMaterial` shader should call `encode_output` the same way.

```wesl
import bevy_ui_render::ui::encode_output;

return encode_output(color);
```

`UiMaterialKey`, `UiPipelineKey`, `BoxShadowPipelineKey`, `UiTextureSlicePipelineKey`,
and `UiGradientPipelineKey` have a new `writer_encode: UiWriterEncodeKey` field. Code
that constructs these keys must set it, with `UiWriterEncodeKey::from_resolved_space`
from the view's `Option<&ResolvedCompositingSpace>`.
